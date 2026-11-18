//! Owned native work barriers. Closing admission never detaches unfinished work.
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Inner { closed: bool, failed: bool, threads: Vec<JoinHandle<()>> }
#[derive(Default)]
pub(crate) struct WorkerGroup { inner: Mutex<Inner>, active: Arc<AtomicUsize> }
pub(crate) struct WorkPermit(Arc<AtomicUsize>);
impl Drop for WorkPermit { fn drop(&mut self) { self.0.fetch_sub(1, Ordering::AcqRel); } }
impl WorkerGroup {
    pub fn enter(&self) -> Result<WorkPermit, String> {
        let inner = self.inner.lock().map_err(|_| "worker admission unavailable")?;
        if inner.closed { return Err("worker admission is closed".into()); }
        self.active.fetch_add(1, Ordering::AcqRel);
        Ok(WorkPermit(self.active.clone()))
    }
    pub fn spawn(&self, work: impl FnOnce() + Send + 'static) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|_| "worker ownership unavailable")?;
        if inner.closed { return Err("worker admission is closed".into()); }
        // Reap only finished workers; never wait for native work under this lock.
        let mut index = 0;
        while index < inner.threads.len() {
            if inner.threads[index].is_finished() {
                if inner.threads.swap_remove(index).join().is_err() { inner.failed = true; }
            } else { index += 1; }
        }
        inner.threads.push(std::thread::Builder::new().spawn(work).map_err(|_| "worker could not start")?);
        Ok(())
    }
    pub fn close(&self) -> Result<(), String> { self.inner.lock().map_err(|_| "worker ownership unavailable")?.closed = true; Ok(()) }
    pub fn drain(&self) -> Result<(), String> {
        self.close()?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            {
                let mut inner = self.inner.lock().map_err(|_| "worker ownership unavailable")?;
                let mut index = 0;
                while index < inner.threads.len() {
                    if inner.threads[index].is_finished() {
                        if inner.threads.swap_remove(index).join().is_err() { inner.failed = true; }
                    } else { index += 1; }
                }
                if inner.failed { return Err("worker completion failed; ownership retained".into()); }
                if inner.threads.is_empty() && self.active.load(Ordering::Acquire) == 0 { return Ok(()); }
            }
            if Instant::now() >= deadline { return Err("workers still draining; retry retained ownership".into()); }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
