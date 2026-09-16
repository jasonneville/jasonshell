//! Deterministic P04 save/restart model. Test-only; never wired to product IPC.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum Phase {
    Prepare,
    Stage,
    WriteBatch,
    FlushClose,
    Revalidate,
    BeforePublish,
    AfterReplace,
    Inspect,
    JournalCommit,
    Cleanup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RestartClass {
    NotStarted,
    Staged,
    PublicationPossible,
    Published,
    Ambiguous,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Disposition {
    AlreadyPublished,
    Cancelled,
    ManualRecoveryRequired,
}

impl Disposition {
    pub(crate) fn from_restart(class: RestartClass, cancellation_requested: bool) -> Self {
        match class {
            RestartClass::Published => Self::AlreadyPublished,
            RestartClass::NotStarted if cancellation_requested => Self::Cancelled,
            _ => Self::ManualRecoveryRequired,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Revisions {
    accepted: u64,
    durable: u64,
    saved: u64,
}

impl Revisions {
    pub(crate) fn new(revision: u64) -> Self {
        Self {
            accepted: revision,
            durable: revision,
            saved: revision,
        }
    }
    pub(crate) fn accept(&mut self) -> u64 {
        self.accepted += 1;
        self.accepted
    }
    pub(crate) fn mark_durable(&mut self, revision: u64) -> Result<(), &'static str> {
        if revision > self.accepted || revision < self.durable {
            return Err("invalid durable revision");
        }
        self.durable = revision;
        Ok(())
    }
    pub(crate) fn mark_saved(&mut self, revision: u64) -> Result<(), &'static str> {
        if revision > self.durable || revision < self.saved {
            return Err("invalid saved revision");
        }
        self.saved = revision;
        Ok(())
    }
    pub(crate) fn accepted(&self) -> u64 {
        self.accepted
    }
    pub(crate) fn durable(&self) -> u64 {
        self.durable
    }
    pub(crate) fn saved(&self) -> u64 {
        self.saved
    }
    pub(crate) fn is_dirty(&self) -> bool {
        self.accepted != self.saved
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Assets {
    pub(crate) target: bool,
    pub(crate) staging: bool,
    pub(crate) backup: bool,
    pub(crate) journal_committed: bool,
    pub(crate) inspected_revision: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DurableJournal {
    version: u32,
    last_durable_phase: Phase,
    assets: Assets,
}

impl DurableJournal {
    pub(crate) fn new(last_durable_phase: Phase, assets: Assets) -> Self {
        Self {
            version: 1,
            last_durable_phase,
            assets,
        }
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub(crate) fn restart_class(&self) -> RestartClass {
        classify(Some(self.last_durable_phase), &self.assets)
    }

    /// Restart never repeats publication. Operator/session must resolve state.
    pub(crate) fn may_retry_publication(&self) -> bool {
        false
    }
}

pub(crate) fn classify(last: Option<Phase>, assets: &Assets) -> RestartClass {
    match last {
        None | Some(Phase::Prepare) => RestartClass::NotStarted,
        Some(
            Phase::Stage
            | Phase::WriteBatch
            | Phase::FlushClose
            | Phase::Revalidate
            | Phase::BeforePublish,
        ) => RestartClass::Staged,
        Some(Phase::AfterReplace) => RestartClass::PublicationPossible,
        Some(Phase::Inspect | Phase::JournalCommit | Phase::Cleanup)
            if assets.target
                && assets.backup
                && assets.inspected_revision
                && assets.journal_committed =>
        {
            RestartClass::Published
        }
        Some(Phase::Inspect | Phase::JournalCommit | Phase::Cleanup) => RestartClass::Ambiguous,
    }
}

pub(crate) trait SaveIo {
    type Error;
    fn step(&mut self, phase: Phase) -> Result<(), Self::Error>;
}

pub(crate) fn run_until_failure<I: SaveIo>(io: &mut I) -> Result<(), (Phase, I::Error)> {
    for phase in [
        Phase::Prepare,
        Phase::Stage,
        Phase::WriteBatch,
        Phase::FlushClose,
        Phase::Revalidate,
        Phase::BeforePublish,
        Phase::AfterReplace,
        Phase::Inspect,
        Phase::JournalCommit,
        Phase::Cleanup,
    ] {
        io.step(phase).map_err(|error| (phase, error))?;
    }
    Ok(())
}
