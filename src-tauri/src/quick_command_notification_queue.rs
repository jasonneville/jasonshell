//! Pure, bounded notification state. The caller owns effects and supplies time.

use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub(crate) const BANNER_LIFETIME: Duration = Duration::from_secs(6);
pub(crate) const READINESS_LIMIT: Duration = Duration::from_secs(2);
const WAITING_LIMIT: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BannerStatus {
    Success,
    Failure,
}

impl BannerStatus {
    pub(crate) fn eligible(
        exit_code: Option<i32>,
        stopped: bool,
        hidden: bool,
        exiting: bool,
    ) -> Option<Self> {
        if stopped || !hidden || exiting {
            return None;
        }
        exit_code.map(|code| {
            if code == 0 {
                Self::Success
            } else {
                Self::Failure
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunTarget {
    pub(crate) command_id: String,
    pub(crate) run_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Banner {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) status: BannerStatus,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BannerAction {
    Open,
    Dismiss,
}

struct Entry {
    banner: Banner,
    target: RunTarget,
}

struct Current {
    entry: Entry,
    phase: Phase,
}

enum Phase {
    Queued,
    Awaiting { deadline: Instant, offered: bool },
    Acknowledged(Instant),
    Shown(Instant),
}

#[derive(Default)]
pub(crate) struct Queue {
    generation: u64,
    current: Option<Current>,
    waiting: VecDeque<Entry>,
    opening: Option<(Entry, Instant)>,
}

impl Queue {
    /// A failed or suppressed history write never reserves a queue slot.
    pub(crate) fn enqueue(
        &mut self,
        target: RunTarget,
        label: String,
        status: BannerStatus,
        saved: bool,
    ) -> bool {
        if !saved || self.entries().any(|entry| entry.target == target) {
            return false;
        }
        let Some(next) = self.generation.checked_add(1) else {
            return false;
        };
        self.generation = next;
        let entry = Entry {
            banner: Banner {
                id: next.to_string(),
                label,
                status,
            },
            target,
        };
        if self.current.is_none() && self.opening.is_none() {
            self.current = Some(Current {
                entry,
                phase: Phase::Queued,
            });
        } else {
            if self.waiting.len() == WAITING_LIMIT {
                self.waiting.pop_front();
            }
            self.waiting.push_back(entry);
        }
        true
    }

    fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.current
            .iter()
            .map(|current| &current.entry)
            .chain(self.waiting.iter())
            .chain(self.opening.iter().map(|(entry, _)| entry))
    }

    /// Start bounded readiness waiting even if the renderer never mounts.
    pub(crate) fn offer(&mut self, now: Instant, renderer_ready: bool) -> Option<&Banner> {
        self.expire(now);
        let current = self.current.as_mut()?;
        match &mut current.phase {
            Phase::Queued => {
                current.phase = Phase::Awaiting {
                    deadline: now + READINESS_LIMIT,
                    offered: renderer_ready,
                }
            }
            Phase::Awaiting { offered, .. } => *offered |= renderer_ready,
            _ => {}
        }
        self.presentation()
    }

    pub(crate) fn presentation(&self) -> Option<&Banner> {
        self.current
            .as_ref()
            .filter(|current| {
                matches!(
                    current.phase,
                    Phase::Awaiting { offered: true, .. }
                        | Phase::Acknowledged(_)
                        | Phase::Shown(_)
                )
            })
            .map(|current| &current.entry.banner)
    }

    pub(crate) fn acknowledge(&mut self, id: &str, now: Instant) -> bool {
        let Some(current) = self
            .current
            .as_mut()
            .filter(|current| current.entry.banner.id == id)
        else {
            return false;
        };
        let Phase::Awaiting {
            offered: true,
            deadline,
        } = current.phase
        else {
            return false;
        };
        if now >= deadline {
            return false;
        }
        current.phase = Phase::Acknowledged(deadline);
        true
    }

    pub(crate) fn ready_to_show(&self, now: Instant) -> Option<&Banner> {
        self.current
            .as_ref()
            .filter(
                |current| matches!(current.phase, Phase::Acknowledged(deadline) if now < deadline),
            )
            .map(|current| &current.entry.banner)
    }

    pub(crate) fn banner(&self) -> Option<&Banner> {
        self.current
            .as_ref()
            .filter(|current| matches!(current.phase, Phase::Shown(_)))
            .map(|current| &current.entry.banner)
    }

    pub(crate) fn settle_show(&mut self, id: &str, shown: bool, now: Instant) -> bool {
        let Some(current) = self.current.as_mut().filter(|current| {
            matches!(current.phase, Phase::Acknowledged(deadline) if now < deadline)
                && current.entry.banner.id == id
        }) else {
            return false;
        };
        if shown {
            current.phase = Phase::Shown(now + BANNER_LIFETIME);
        } else {
            self.advance();
        }
        shown
    }

    pub(crate) fn wake_at(&self) -> Option<Instant> {
        self.current
            .as_ref()
            .and_then(|current| match current.phase {
                Phase::Awaiting { deadline, .. }
                | Phase::Acknowledged(deadline)
                | Phase::Shown(deadline) => Some(deadline),
                Phase::Queued => None,
            })
            .or_else(|| self.opening.as_ref().map(|(_, deadline)| *deadline))
    }

    pub(crate) fn expire(&mut self, now: Instant) {
        if self.wake_at().is_some_and(|deadline| now >= deadline) {
            self.opening = None;
            self.advance();
        }
    }

    pub(crate) fn act(&mut self, id: &str, action: BannerAction, now: Instant) -> bool {
        if !self.current.as_ref().is_some_and(|current| {
            current.entry.banner.id == id
                && matches!(current.phase, Phase::Shown(deadline) if now < deadline)
        }) {
            return false;
        }
        match action {
            BannerAction::Dismiss => self.advance(),
            BannerAction::Open => {
                self.opening = self
                    .current
                    .take()
                    .map(|current| (current.entry, now + BANNER_LIFETIME));
            }
        }
        true
    }

    pub(crate) fn claim_open(&mut self, id: &str, now: Instant) -> Option<RunTarget> {
        if !self
            .opening
            .as_ref()
            .is_some_and(|(entry, deadline)| entry.banner.id == id && now < *deadline)
        {
            return None;
        }
        let target = self.opening.take()?.0.target;
        self.advance();
        Some(target)
    }

    pub(crate) fn clear(&mut self) {
        self.current = None;
        self.waiting.clear();
        self.opening = None;
    }

    fn advance(&mut self) {
        self.current = self.waiting.pop_front().map(|entry| Current {
            entry,
            phase: Phase::Queued,
        });
    }
}
