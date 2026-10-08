//! Shared production ownership seam for live registrations and fake Drop probes.
//! Methods transfer retired resources to callers; native Drop never runs while
//! a caller holds the gallery runtime mutex.
use std::collections::{BTreeMap, BTreeSet};

/// An owned visible-frame set. Each resource has exactly one owner.
pub struct ResourceSet<R> {
    entries: BTreeMap<String, R>,
}

impl<R> Default for ResourceSet<R> {
    fn default() -> Self { Self { entries: BTreeMap::new() } }
}

impl<R> ResourceSet<R> {
    /// Transfer a replaced registration to the caller for disposal.
    pub fn replace(&mut self, hwnd: String, resource: R) -> Option<R> {
        self.entries.insert(hwnd, resource)
    }

    /// Transfer a registration for reuse or explicit removal.
    pub fn remove(&mut self, hwnd: &str) -> Option<R> {
        self.entries.remove(hwnd)
    }

    /// Retain only this request's visible HWNDs; transfer omissions for disposal.
    pub fn retain_visible<'a>(&mut self, visible: impl IntoIterator<Item = &'a str>) -> Self {
        let visible: BTreeSet<&str> = visible.into_iter().collect();
        let mut omitted = Self::default();
        for (hwnd, resource) in std::mem::take(&mut self.entries) {
            if visible.contains(hwnd.as_str()) {
                self.entries.insert(hwnd, resource);
            } else {
                omitted.entries.insert(hwnd, resource);
            }
        }
        omitted
    }

    /// Inspect ownership without cloning resources.
    pub fn get(&self, hwnd: &str) -> Option<&R> { self.entries.get(hwnd) }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

/// Completion transfers either the replaced committed set or the stale candidate
/// to the caller. Dropping `retired` cannot affect the currently committed set.
pub struct ResourceCompletion<R> {
    pub accepted: bool,
    pub retired: ResourceSet<R>,
}

/// Generation-scoped committed registrations. Admission remains the caller's
/// responsibility; use a nonce/revision/request token as G in production.
pub struct ResourceOwner<G, R> {
    generation: Option<G>,
    resources: ResourceSet<R>,
}

impl<G, R> Default for ResourceOwner<G, R> {
    fn default() -> Self { Self { generation: None, resources: ResourceSet::default() } }
}

impl<G: PartialEq, R> ResourceOwner<G, R> {
    /// Accept an already-admitted generation. Committed registrations remain
    /// reusable until this generation takes them; stale workers lose authority.
    pub fn advance(&mut self, generation: G) { self.generation = Some(generation); }

    pub fn is_current(&self, generation: &G) -> bool {
        self.generation.as_ref() == Some(generation)
    }

    /// Revoke completion authority immediately, without native work. Cleanup can
    /// wait for the serialized native lane; staged resources remain worker-owned.
    pub fn cancel(&mut self) { self.generation = None; }

    /// Hide, host geometry change, and session reset transfer all committed
    /// resources for disposal and revoke every outstanding candidate.
    pub fn clear(&mut self) -> ResourceSet<R> {
        self.cancel();
        std::mem::take(&mut self.resources)
    }

    /// Only the current worker can take committed registrations for reuse.
    pub fn take_if_current(&mut self, generation: &G) -> Option<ResourceSet<R>> {
        self.is_current(generation).then(|| std::mem::take(&mut self.resources))
    }

    /// Replace the committed set only if the candidate still owns this exact
    /// generation. A stale candidate is returned intact; newer resources survive.
    pub fn finish(&mut self, generation: &G, candidate: ResourceSet<R>) -> ResourceCompletion<R> {
        if self.is_current(generation) {
            ResourceCompletion { accepted: true, retired: std::mem::replace(&mut self.resources, candidate) }
        } else {
            ResourceCompletion { accepted: false, retired: candidate }
        }
    }

    /// Remove a captured member without clearing other registrations.
    pub fn remove(&mut self, hwnd: &str) -> Option<R> { self.resources.remove(hwnd) }

    /// Retire removed or changed members without disturbing surviving resources.
    pub fn retain_visible<'a>(&mut self, visible: impl IntoIterator<Item = &'a str>) -> ResourceSet<R> {
        self.resources.retain_visible(visible)
    }

    pub fn get(&self, hwnd: &str) -> Option<&R> { self.resources.get(hwnd) }
    pub fn len(&self) -> usize { self.resources.len() }
    pub fn is_empty(&self) -> bool { self.resources.is_empty() }
}
