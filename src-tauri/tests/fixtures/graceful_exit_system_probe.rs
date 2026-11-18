//! Test-only setup inside the actual native runtime. The child executor, runtime
//! stop flag, retained JoinHandle ledger and drain implementation are production.
use super::*;
use crate::process_manager::probe_executor::{execute_command_observed, ProbeError, ProbeEvent};
use std::{path::Path, process::Command, sync::mpsc};

impl SystemInfoRuntime {
    pub(crate) fn start_owned_exit_probe_for_test(&self, executable: &Path) -> Result<(mpsc::Receiver<ProbeEvent>, Arc<Mutex<Vec<ProbeEvent>>>), String> {
        if self.stop.load(Ordering::Acquire) { return Err("probe runtime already stopped".into()); }
        let (send, receive) = mpsc::channel(); let events = Arc::new(Mutex::new(vec![]));
        let observed = events.clone(); let stop = self.stop.clone(); let executable=executable.to_path_buf();
        let worker=std::thread::spawn(move || {
            let mut command=Command::new(executable);
            command.args(["-NoLogo","-NoProfile","-NonInteractive","-Command","Start-Sleep -Seconds 120"]);
            let result=execute_command_observed(&mut command,||stop.load(Ordering::Acquire),Duration::from_secs(120),1024,|event|{
                observed.lock().unwrap().push(event);let _=send.send(event);
            });
            assert_eq!(result,Err(ProbeError::Cancelled),"production stop flag must cancel actual owned executor");
        });
        self.workers.lock().map_err(|_| "System Info test worker ledger poisoned")?.push(worker);
        Ok((receive,events))
    }
}
