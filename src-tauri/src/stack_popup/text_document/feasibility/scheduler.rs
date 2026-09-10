//! Three fixed workers: two exclusively urgent, one background. A blocked scan
//! cannot consume either urgent worker. Lock scope never includes work or IPC.
use super::contract::{ProtocolErrorCode, IO_BLOCK_BYTES, MAX_IN_FLIGHT_RANGE_PAYLOADS};
use super::{checked_add, failure, Result};
use serde::Serialize;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Condvar, LockResult, Mutex, MutexGuard, PoisonError,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

thread_local! {
    static ACTOR_LOCK_DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn enter_actor_lock() {
    ACTOR_LOCK_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
}

fn leave_actor_lock() {
    ACTOR_LOCK_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
}

fn actor_lock_held_by_current_thread() -> bool {
    ACTOR_LOCK_DEPTH.with(|depth| depth.get() != 0)
}

type Work = Box<dyn FnOnce(&AtomicBool) -> Result<Vec<u8>> + Send + 'static>;
pub(crate) const MAX_QUEUED_JOBS: usize = 4;
const PRIORITY_COUNT: usize = 5;
struct Job {
    id: u64,
    cancel: Arc<AtomicBool>,
    work: Work,
    sender: SyncSender<Result<Payload>>,
    priority: Priority,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) enum Priority {
    Demand,
    Prefetch,
    Index,
    Compaction,
    Background,
}
impl Priority {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Demand => 0,
            Self::Prefetch => 1,
            Self::Index => 2,
            Self::Compaction => 3,
            Self::Background => 4,
        }
    }
    fn background_slot(self) -> usize {
        match self {
            Self::Prefetch => 0,
            Self::Index | Self::Background => 1,
            Self::Compaction => 2,
            Self::Demand => unreachable!("demand is not a background lane"),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub(crate) enum CancelOutcome {
    QueuedRemoved,
    RequestedDriverDependent,
    FinishedOrUnknown,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub(crate) struct SchedulerMetrics {
    pub queued_peak: usize,
    pub active_peak: usize,
    pub payload_peak: usize,
    pub superseded: u64,
    pub actor_lock_max_ns: u64,
    pub actor_lock_entries: u64,
    pub actor_lock_io_checks: u64,
    pub actor_lock_io_violations: u64,
    pub completed: u64,
    pub priority_completed: [u64; PRIORITY_COUNT],
}
struct State {
    latest: Option<Job>,
    background: [Option<Job>; 3],
    active: [Option<(u64, Arc<AtomicBool>)>; 3],
    credits: usize,
    next: u64,
    closed: bool,
    metrics: SchedulerMetrics,
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    actor_lock_max_ns: AtomicU64,
    actor_lock_entries: AtomicU64,
    actor_lock_io_checks: AtomicU64,
    actor_lock_io_violations: AtomicU64,
}

struct ActorGuard<'a> {
    shared: &'a Shared,
    guard: Option<MutexGuard<'a, State>>,
    started: Instant,
    locked: bool,
}

impl<'a> ActorGuard<'a> {
    fn new(shared: &'a Shared, guard: MutexGuard<'a, State>) -> Self {
        enter_actor_lock();
        shared.actor_lock_entries.fetch_add(1, Ordering::Relaxed);
        Self {
            shared,
            guard: Some(guard),
            started: Instant::now(),
            locked: true,
        }
    }

    fn release_lock(&mut self) {
        if self.locked {
            self.locked = false;
            self.shared.record_actor_lock_duration(self.started.elapsed());
            leave_actor_lock();
        }
    }

    fn wait(mut self) -> LockResult<Self> {
        let shared = self.shared;
        let guard = self.guard.take().expect("actor guard has a mutex guard");
        self.release_lock();
        drop(self);
        match shared.changed.wait(guard) {
            Ok(guard) => Ok(Self::new(shared, guard)),
            Err(error) => Err(PoisonError::new(Self::new(shared, error.into_inner()))),
        }
    }
}

impl Deref for ActorGuard<'_> {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().expect("actor guard has a mutex guard")
    }
}

impl DerefMut for ActorGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().expect("actor guard has a mutex guard")
    }
}

impl Drop for ActorGuard<'_> {
    fn drop(&mut self) {
        self.release_lock();
    }
}

impl Shared {
    fn lock_state(&self) -> Result<ActorGuard<'_>> {
        self.state
            .lock()
            .map(|guard| ActorGuard::new(self, guard))
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "scheduler lock poisoned"))
    }

    fn record_io_boundary(&self) -> Result<()> {
        self.actor_lock_io_checks.fetch_add(1, Ordering::Relaxed);
        if actor_lock_held_by_current_thread() {
            self.actor_lock_io_violations
                .fetch_add(1, Ordering::Relaxed);
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "scheduler attempted work while actor lock was held",
            ));
        }
        Ok(())
    }

    fn record_actor_lock_duration(&self, duration: std::time::Duration) {
        let nanos = duration.as_nanos().min(u64::MAX as u128) as u64;
        let mut current = self.actor_lock_max_ns.load(Ordering::Relaxed);
        while nanos > current {
            match self.actor_lock_max_ns.compare_exchange_weak(
                current,
                nanos,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
    }
}

pub(crate) struct Payload {
    pub bytes: Vec<u8>,
    shared: Arc<Shared>,
}
impl Drop for Payload {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.credits = state.credits.saturating_sub(1);
            self.shared.changed.notify_all();
        }
    }
}
pub(crate) struct Ticket {
    pub id: u64,
    pub result: Receiver<Result<Payload>>,
}
pub(crate) struct Scheduler {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
}
impl Scheduler {
    pub(crate) fn new() -> Result<Self> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                latest: None,
                background: std::array::from_fn(|_| None),
                active: std::array::from_fn(|_| None),
                credits: 0,
                next: 1,
                closed: false,
                metrics: SchedulerMetrics::default(),
            }),
            changed: Condvar::new(),
            actor_lock_max_ns: AtomicU64::new(0),
            actor_lock_entries: AtomicU64::new(0),
            actor_lock_io_checks: AtomicU64::new(0),
            actor_lock_io_violations: AtomicU64::new(0),
        });
        let mut scheduler = Self {
            shared,
            workers: Vec::with_capacity(3),
        };
        for worker in 0..3 {
            let shared = scheduler.shared.clone();
            match thread::Builder::new()
                .name(format!("text-p02-{worker}"))
                .spawn(move || run_worker(shared, worker))
            {
                Ok(handle) => scheduler.workers.push(handle),
                Err(error) => {
                    scheduler.close();
                    return Err(super::io_failure(error));
                }
            }
        }
        Ok(scheduler)
    }
    pub(crate) fn submit(
        &self,
        priority: Priority,
        work: impl FnOnce(&AtomicBool) -> Result<Vec<u8>> + Send + 'static,
    ) -> Result<Ticket> {
        let (sender, receiver) = sync_channel(1);
        let mut state = self.shared.lock_state()?;
        if state.closed {
            return Err(failure(ProtocolErrorCode::Cancelled, "scheduler closed"));
        }
        let id = state.next;
        state.next = checked_add(id, 1)?;
        let job = Job {
            id,
            cancel: Arc::new(AtomicBool::new(false)),
            work: Box::new(work),
            sender,
            priority,
        };
        let previous = match priority {
            Priority::Demand => state.latest.replace(job),
            _ => {
                let slot = priority.background_slot();
                state.background[slot].replace(job)
            }
        };
        if previous.is_some() {
            state.metrics.superseded += 1;
        }
        state.metrics.queued_peak = state.metrics.queued_peak.max(
            usize::from(state.latest.is_some())
                + state.background.iter().filter(|job| job.is_some()).count(),
        );
        drop(state);
        if let Some(previous) = previous {
            previous.cancel.store(true, Ordering::Release);
            let _ = previous.sender.try_send(Err(failure(
                ProtocolErrorCode::Cancelled,
                "queued demand superseded",
            )));
        }
        self.shared.changed.notify_all();
        Ok(Ticket {
            id,
            result: receiver,
        })
    }
    pub(crate) fn cancel(&self, id: u64) -> Result<CancelOutcome> {
        let mut state = self.shared.lock_state()?;
        let queued = if state.latest.as_ref().is_some_and(|job| job.id == id) {
            state.latest.take()
        } else if let Some(slot) = state
            .background
            .iter()
            .position(|job| job.as_ref().is_some_and(|job| job.id == id))
        {
            state.background[slot].take()
        } else {
            None
        };
        let outcome = if queued.is_some() {
            CancelOutcome::QueuedRemoved
        } else if let Some((_, cancel)) = state
            .active
            .iter()
            .flatten()
            .find(|(active, _)| *active == id)
        {
            cancel.store(true, Ordering::Release);
            CancelOutcome::RequestedDriverDependent
        } else {
            CancelOutcome::FinishedOrUnknown
        };
        drop(state);
        if let Some(job) = queued {
            job.cancel.store(true, Ordering::Release);
            let _ = job.sender.try_send(Err(failure(
                ProtocolErrorCode::Cancelled,
                "queued work removed",
            )));
        }
        self.shared.changed.notify_all();
        Ok(outcome)
    }
    pub(crate) fn metrics(&self) -> Result<SchedulerMetrics> {
        let mut state = self.shared.lock_state()?;
        state.metrics.actor_lock_max_ns = self.shared.actor_lock_max_ns.load(Ordering::Relaxed);
        state.metrics.actor_lock_entries = self.shared.actor_lock_entries.load(Ordering::Relaxed);
        state.metrics.actor_lock_io_checks = self.shared.actor_lock_io_checks.load(Ordering::Relaxed);
        state.metrics.actor_lock_io_violations = self
            .shared
            .actor_lock_io_violations
            .load(Ordering::Relaxed);
        Ok(state.metrics)
    }
    fn close(&mut self) {
        let queued = if let Ok(mut state) = self.shared.lock_state() {
            state.closed = true;
            for (_, cancel) in state.active.iter().flatten() {
                cancel.store(true, Ordering::Release);
            }
            [
                state.latest.take(),
                state.background[0].take(),
                state.background[1].take(),
                state.background[2].take(),
            ]
        } else {
            [None, None, None, None]
        };
        for job in queued.into_iter().flatten() {
            let _ = job.sender.try_send(Err(failure(
                ProtocolErrorCode::Cancelled,
                "scheduler disposed",
            )));
        }
        self.shared.changed.notify_all();
    }
}
impl Drop for Scheduler {
    fn drop(&mut self) {
        self.close();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
fn run_worker(shared: Arc<Shared>, worker: usize) {
    loop {
        let job = {
            let Ok(mut state) = shared.lock_state() else {
                return;
            };
            loop {
                if state.closed {
                    return;
                }
                let available = if worker < 2 {
                    state.latest.is_some()
                } else {
                    state.background.iter().any(Option::is_some)
                };
                // Reserve one credit for urgent work even when a background payload is retained.
                let credit = state.credits < MAX_IN_FLIGHT_RANGE_PAYLOADS
                    && (worker < 2 || state.credits < MAX_IN_FLIGHT_RANGE_PAYLOADS - 1);
                if available && credit {
                    let job = if worker < 2 {
                        state.latest.take()
                    } else {
                        state.background.iter_mut().find_map(Option::take)
                    };
                    if let Some(job) = job {
                        state.credits += 1;
                        state.active[worker] = Some((job.id, job.cancel.clone()));
                        state.metrics.payload_peak = state.metrics.payload_peak.max(state.credits);
                        state.metrics.active_peak = state
                            .metrics
                            .active_peak
                            .max(state.active.iter().flatten().count());
                        break job;
                    }
                }
                match state.wait() {
                    Ok(next) => state = next,
                    Err(_) => return,
                }
            }
        };
        // Work owns no actor guard. Byte vector admission is bounded before delivery.
        let result = shared
            .record_io_boundary()
            .and_then(|_| (job.work)(&job.cancel))
            .and_then(|bytes| {
                if job.cancel.load(Ordering::Acquire) {
                    Err(failure(
                        ProtocolErrorCode::Cancelled,
                        "in-flight work completed after cancel request",
                    ))
                } else if bytes.len() > IO_BLOCK_BYTES {
                    Err(failure(
                        ProtocolErrorCode::ResourceLimit,
                        "worker payload exceeds bounded credit",
                    ))
                } else {
                    Ok(bytes)
                }
            });
        if let Ok(mut state) = shared.lock_state() {
            state.active[worker] = None;
            state.metrics.completed += 1;
            state.metrics.priority_completed[job.priority.index()] += 1;
            if result.is_err() {
                state.credits = state.credits.saturating_sub(1);
            }
        }
        let delivered = result.map(|bytes| Payload {
            bytes,
            shared: shared.clone(),
        });
        // try_send never waits for a receiver; dropped payload returns its credit.
        let _ = job.sender.try_send(delivered);
        shared.changed.notify_all();
    }
}
