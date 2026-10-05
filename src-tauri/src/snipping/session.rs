//! Single-session ownership and exact concrete-caller binding. No IPC payload is trusted here.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller { pub label: String, pub window_id: u64 }

#[derive(Clone, Copy, Debug)]
pub enum Action { Select, Cancel, Copy, Save, Close }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError { Busy, Exhausted, Stale, Phase, Unauthorized }

enum Phase { Selecting(Caller), Preview(Caller) }
struct Active<R> { generation: u64, _resource: R, phase: Phase }

pub struct SessionManager<R> { next: u64, active: Option<Active<R>> }

impl<R> Default for SessionManager<R> {
    fn default() -> Self { Self { next: 0, active: None } }
}

impl<R> SessionManager<R> {
    pub fn begin(&mut self, resource: R, overlay: Caller) -> Result<u64, SessionError> {
        if self.active.is_some() { return Err(SessionError::Busy); }
        if overlay.label != "snip-overlay" || overlay.window_id == 0 { return Err(SessionError::Unauthorized); }
        let generation = self.next.checked_add(1).ok_or(SessionError::Exhausted)?;
        self.next = generation;
        self.active = Some(Active { generation, _resource: resource, phase: Phase::Selecting(overlay) });
        Ok(generation)
    }
    pub fn show_preview(&mut self, generation: u64, preview: Caller) -> Result<(), SessionError> {
        let active = self.active.as_mut().filter(|s| s.generation == generation).ok_or(SessionError::Stale)?;
        if !matches!(active.phase, Phase::Selecting(_)) { return Err(SessionError::Phase); }
        if preview.label != "snip-preview" || preview.window_id == 0 { return Err(SessionError::Unauthorized); }
        active.phase = Phase::Preview(preview);
        Ok(())
    }
    pub fn authorize(&self, generation: u64, caller: &Caller, action: Action) -> Result<(), SessionError> {
        let active = self.active.as_ref().filter(|s| s.generation == generation).ok_or(SessionError::Stale)?;
        let allowed = match &active.phase {
            Phase::Selecting(bound) => bound == caller && matches!(action, Action::Select | Action::Cancel),
            Phase::Preview(bound) => bound == caller && matches!(action, Action::Copy | Action::Save | Action::Close),
        };
        if allowed { Ok(()) } else { Err(SessionError::Unauthorized) }
    }
    /// Trusted coordinator only; IPC must authorize caller before invoking this method.
    pub fn cancel(&mut self, generation: u64) -> Result<(), SessionError> {
        if !self.active.as_ref().is_some_and(|s| s.generation == generation) { return Err(SessionError::Stale); }
        self.active.take();
        Ok(())
    }
    /// Trusted coordinator only; releases the owned resource after a preview closes.
    pub fn finish(&mut self, generation: u64) -> Result<(), SessionError> {
        let active = self.active.as_ref().filter(|s| s.generation == generation).ok_or(SessionError::Stale)?;
        if !matches!(active.phase, Phase::Preview(_)) { return Err(SessionError::Phase); }
        self.active.take();
        Ok(())
    }
}
