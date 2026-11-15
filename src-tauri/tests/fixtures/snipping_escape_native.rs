// Opt-in real OS boundary. Included alongside the existing synthetic runtime
// fixture; never runs production main, GDI capture, clipboard or native preview.
mod native_escape_acceptance {
    use super::*;
    use std::{io::{BufRead,Write},process::{Child,Command,Stdio}};
    use windows::Win32::{Foundation::HWND,UI::{WindowsAndMessaging::{GetForegroundWindow,SetForegroundWindow,GetWindowThreadProcessId},Input::KeyboardAndMouse::{SendInput,INPUT,INPUT_0,INPUT_KEYBOARD,KEYBDINPUT,KEYEVENTF_KEYUP,VK_ESCAPE}}};
    const TARGET:&str="snipping::runtime::runtime_acceptance::native_escape_acceptance::snip_native_escape_external_target";
    struct OwnedChild(Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            // Kill/wait only our exact child, including assertion unwind. Windows
            // releases its owned HWND/class when this process exits.
            if self.0.try_wait().ok().flatten().is_none(){let _=self.0.kill();}
            let _=self.0.wait();
        }
    }
    struct Listener(Arc<EventRouter>);
    impl Drop for Listener {fn drop(&mut self){self.0.shutdown();}}
    fn input(up:bool)->u32 {
        let event=INPUT {r#type:INPUT_KEYBOARD,Anonymous:INPUT_0 {ki:KEYBDINPUT {
            wVk:VK_ESCAPE,dwFlags:if up {KEYEVENTF_KEYUP}else{Default::default()},..Default::default()
        }}};
        // SAFETY: valid single stack-owned INPUT for synchronous SendInput.
        unsafe {SendInput(&[event],std::mem::size_of::<INPUT>() as i32)}
    }
    struct KeyRelease(bool);
    impl Drop for KeyRelease {fn drop(&mut self){if self.0 {let _=input(true);}}}
    fn metadata(text:&str){println!("\n{text}");std::io::stdout().flush().unwrap();}

    #[test]
    #[ignore="explicit JASONSHELL_SNIP_ESCAPE_NATIVE_TESTS=1; owned window, production keyboard listener, real Escape only"]
    fn snip_native_escape_external_target() {
        assert_eq!(std::env::var("JASONSHELL_SNIP_ESCAPE_NATIVE_TESTS").as_deref(),Ok("1"));
        assert_eq!(std::env::var("JASONSHELL_SNIP_ESCAPE_TARGET_CHILD").as_deref(),Ok("1"));
        let _dpi=crate::snipping::windows::PhysicalDpi::enter().unwrap();
        let class=crate::snipping::windows::WindowClass::register().unwrap();
        let window=crate::snipping::windows::NativeWindow::create(&class,PhysicalRect {left:100,top:100,right:500,bottom:300},crate::snipping::windows::Kind::SyntheticTarget).unwrap();
        window.show().unwrap();
        unsafe {let _=SetForegroundWindow(window.hwnd());}
        metadata(&format!("SNIP_ESCAPE_TARGET_READY={}",window.hwnd().0 as isize));
        let (eof,closed)=mpsc::channel();
        thread::spawn(move||{let mut line=String::new();let _=std::io::stdin().read_line(&mut line);let _=eof.send(());});
        let deadline=Instant::now()+Duration::from_secs(15);let mut reported=false;
        while Instant::now()<deadline && closed.try_recv().is_err() {
            crate::snipping::windows::pump(Duration::from_millis(10));
            if !reported && window.pointer_observation().escape {
                reported=true;metadata("SNIP_ESCAPE_TARGET_RECEIVED=1");
            }
        }
        // NativeWindow/Class RAII run on their constructing thread.
    }

    #[test]
    #[ignore="explicit consent: temporary external window + actual production hook + injected Escape; no capture/clipboard/shell"]
    fn snip_native_escape_installed_listener_cancels_external_target_pulse() {
        assert_eq!(std::env::var("JASONSHELL_SNIP_ESCAPE_NATIVE_TESTS").as_deref(),Ok("1"));
        let f=Fixture::new();let t=f.accepted();escape_overlay(&f,&t);
        let install=crate::snipping::escape::install(Arc::downgrade(&f.runtime));
        assert!(install.is_ok(),"production escape::install failed: {:?}",install.as_ref().err());
        let listener=Listener(install.unwrap());
        metadata("SNIP_ESCAPE_PRODUCTION_LISTENER_INSTALLED=1");
        run_native_pulse(f,listener);
    }
    fn run_native_pulse(f:Fixture,listener:Listener) {
        let mut child=OwnedChild(Command::new(std::env::current_exe().unwrap())
            .args(["--ignored","--exact",TARGET,"--test-threads=1","--nocapture"])
            .env("JASONSHELL_SNIP_ESCAPE_TARGET_CHILD","1")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap());
        let stdout=child.0.stdout.take().unwrap();let (reports,report_rx)=mpsc::channel();
        let reader=thread::spawn(move||{
            for line in std::io::BufReader::new(stdout).lines().take(24) {
                let Ok(line)=line else{break;};if line.len()>512{break;}
                if reports.send(line).is_err(){break;}
            }
        });
        let ready_deadline=Instant::now()+Duration::from_secs(5);
        let hwnd=loop {
            let line=report_rx.recv_timeout(ready_deadline.saturating_duration_since(Instant::now())).expect("owned external target readiness");
            if let Some(value)=line.strip_prefix("SNIP_ESCAPE_TARGET_READY=") {break HWND(value.parse::<isize>().unwrap() as *mut _);}
        };
        let mut pid=0;unsafe {GetWindowThreadProcessId(hwnd,Some(&mut pid));}
        assert_eq!(pid,child.0.id(),"only exact owned external target may receive input");
        let foreground_deadline=Instant::now()+Duration::from_secs(2);
        while unsafe {GetForegroundWindow()}!=hwnd && Instant::now()<foreground_deadline {
            unsafe {let _=SetForegroundWindow(hwnd);}
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(unsafe {GetForegroundWindow()},hwnd,"abort before injection unless owned target has foreground");
        let mut release=KeyRelease(false);
        assert_eq!(input(false),1,"SendInput keydown rejected (possible UIPI boundary)");release.0=true;
        thread::sleep(Duration::from_millis(20));
        assert_eq!(input(true),1,"SendInput keyup rejected");release.0=false;
        let deadline=Instant::now()+Duration::from_secs(2);
        while Instant::now()<deadline && f.runtime.coordinator.inspect().phase.is_some(){thread::sleep(Duration::from_millis(5));}
        let observed=f.runtime.coordinator.inspect().phase;
        let unchanged=unsafe {GetForegroundWindow()}==hwnd;
        let mut target_received=false;
        while let Ok(line)=report_rx.try_recv(){target_received|=line=="SNIP_ESCAPE_TARGET_RECEIVED=1";}
        // Target may receive the forwarded event just after coordinator delivery.
        if !target_received {if let Ok(line)=report_rx.recv_timeout(Duration::from_millis(250)){target_received=line=="SNIP_ESCAPE_TARGET_RECEIVED=1";}}
        let direct=if observed.is_some(){
            let router=EventRouter::new(Arc::downgrade(&f.runtime),||{});
            router.route(WM_KEYDOWN,ESCAPE_KEY).is_some_and(|event|event.deliver())
        }else{false};
        // Shut down listener and exact target BEFORE behavioral assertions. The
        // hosting test process then exits, removing any outstanding native hook.
        listener.0.shutdown();drop(listener);
        drop(child.0.stdin.take());
        let target_deadline=Instant::now()+Duration::from_secs(2);
        while child.0.try_wait().unwrap().is_none() && Instant::now()<target_deadline {thread::sleep(Duration::from_millis(10));}
        let exited_normally=child.0.try_wait().unwrap().is_some_and(|status|status.success());drop(child);reader.join().unwrap();
        metadata(&format!("SNIP_ESCAPE_NATIVE_RESULT pulse_ms=20 observed_phase={observed:?} foreground_unchanged={unchanged} target_received={target_received} direct_router_fallback={direct} owned_target_exited={exited_normally}"));
        assert!(exited_normally,"owned target must exit after EOF before report");
        assert!(unchanged,"production cancellation must not change foreground");
        assert!(target_received,"external window must observe forwarded Escape");
        assert!(observed.is_none(),"production install + real 20ms Escape left {observed:?} after2s; direct_router_fallback={direct}");
        f.clean();assert_eq!(f.count("close"),1);assert_eq!(f.count("restore"),0);
    }
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snipping_escape_webview_lifecycle.rs"));
}
