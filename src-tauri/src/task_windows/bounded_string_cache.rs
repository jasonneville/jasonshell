use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::time::{Duration, Instant};

pub(crate) struct BoundedStringCache<K> {
    entries: HashMap<K, CacheEntry>,
    order: VecDeque<K>,
    capacity: usize,
    positive_ttl: Duration,
    negative_ttl: Duration,
}

struct CacheEntry {
    value: Option<String>,
    inserted_at: Instant,
}

impl<K> BoundedStringCache<K>
where
    K: Eq + Hash + Clone,
{
    pub(crate) fn new(capacity: usize, positive_ttl: Duration, negative_ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
            positive_ttl,
            negative_ttl,
        }
    }

    pub(crate) fn get_cloned(&mut self, key: &K) -> Option<Option<String>> {
        self.evict_expired();
        if let Some(cached) = self.entries.get(key).map(|entry| entry.value.clone()) {
            self.touch(key);
            return Some(cached);
        }
        None
    }

    pub(crate) fn insert(&mut self, key: K, value: Option<String>) {
        self.insert_internal(key, value);
    }

    fn insert_internal(&mut self, key: K, value: Option<String>) {
        self.order.retain(|entry_key| entry_key != &key);
        self.entries.insert(
            key.clone(),
            CacheEntry {
                value,
                inserted_at: Instant::now(),
            },
        );
        self.order.push_back(key);
        self.evict_over_capacity();
    }

    fn touch(&mut self, key: &K) {
        self.order.retain(|entry_key| entry_key != key);
        self.order.push_back(key.clone());
    }

    fn evict_expired(&mut self) {
        let now = Instant::now();
        let keys: Vec<K> = self
            .order
            .iter()
            .filter(|key| {
                self.entries.get(*key).is_some_and(|entry| {
                    entry.is_expired(now, self.positive_ttl, self.negative_ttl)
                })
            })
            .cloned()
            .collect();
        for key in keys {
            self.remove(&key);
        }
    }

    fn evict_over_capacity(&mut self) {
        while self.entries.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            } else {
                break;
            }
        }
    }

    fn remove(&mut self, key: &K) {
        self.entries.remove(key);
        self.order.retain(|entry_key| entry_key != key);
    }
}

impl CacheEntry {
    fn is_expired(&self, now: Instant, positive_ttl: Duration, negative_ttl: Duration) -> bool {
        let ttl = if self.value.is_some() {
            positive_ttl
        } else {
            negative_ttl
        };
        now.duration_since(self.inserted_at) >= ttl
    }
}

#[cfg(test)]
mod tests {
    use super::BoundedStringCache;
    use std::time::Duration;

    #[test]
    fn evicts_oldest_entry_at_capacity() {
        let mut cache = BoundedStringCache::new(2, Duration::from_secs(60), Duration::from_secs(5));
        cache.insert("a".to_string(), Some("one".to_string()));
        cache.insert("b".to_string(), Some("two".to_string()));
        cache.insert("c".to_string(), Some("three".to_string()));

        assert_eq!(cache.get_cloned(&"a".to_string()), None);
        assert_eq!(
            cache.get_cloned(&"b".to_string()),
            Some(Some("two".to_string()))
        );
        assert_eq!(
            cache.get_cloned(&"c".to_string()),
            Some(Some("three".to_string()))
        );
    }

    #[test]
    fn reinserting_key_updates_recency() {
        let mut cache = BoundedStringCache::new(2, Duration::from_secs(60), Duration::from_secs(5));
        cache.insert("a".to_string(), Some("one".to_string()));
        cache.insert("b".to_string(), Some("two".to_string()));
        cache.insert("a".to_string(), Some("one-again".to_string()));
        cache.insert("c".to_string(), Some("three".to_string()));

        assert_eq!(cache.get_cloned(&"b".to_string()), None);
        assert_eq!(
            cache.get_cloned(&"a".to_string()),
            Some(Some("one-again".to_string()))
        );
        assert_eq!(
            cache.get_cloned(&"c".to_string()),
            Some(Some("three".to_string()))
        );
    }

    #[test]
    fn caches_positive_and_negative_values() {
        let mut cache = BoundedStringCache::new(2, Duration::from_secs(60), Duration::from_secs(5));
        cache.insert("hit".to_string(), Some("icon".to_string()));
        cache.insert("miss".to_string(), None);

        assert_eq!(
            cache.get_cloned(&"hit".to_string()),
            Some(Some("icon".to_string()))
        );
        assert_eq!(cache.get_cloned(&"miss".to_string()), Some(None));
    }

    #[test]
    fn expires_positive_entries_after_ttl() {
        let mut cache =
            BoundedStringCache::new(2, Duration::from_millis(10), Duration::from_secs(5));
        cache.insert("hit".to_string(), Some("icon".to_string()));
        std::thread::sleep(Duration::from_millis(20));

        assert_eq!(cache.get_cloned(&"hit".to_string()), None);
    }

    #[test]
    fn expires_negative_entries_after_ttl() {
        let mut cache =
            BoundedStringCache::new(2, Duration::from_secs(5), Duration::from_millis(10));
        cache.insert("miss".to_string(), None);
        std::thread::sleep(Duration::from_millis(20));

        assert_eq!(cache.get_cloned(&"miss".to_string()), None);
    }
}
