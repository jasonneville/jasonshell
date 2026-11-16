// Included inside native_escape_acceptance. Temporary plain hidden WebViews only:
// no product renderer, AppBar, capture, clipboard, network, or shell startup.
use tauri::{Manager,WebviewUrl,WebviewWindowBuilder};
const WEBVIEW_CASE:&str="snipping::runtime::runtime_acceptance::native_escape_acceptance::snip_native_escape_listener_before_webviews_survives_delayed_selection";
struct OwnedWebviewProfile(std::path::PathBuf);
impl Drop for OwnedWebviewProfile {
    fn drop(&mut self){for _ in 0..60 {
        if std::fs::remove_dir_all(&self.0).is_ok() || !self.0.exists(){return;}
        thread::sleep(Duration::from_millis(100));
    }}
}
#[test]
#[ignore="consented temporary WebViews + production keyboard hook + external target/Escape only; explicit native opt-in"]
fn snip_native_escape_listener_before_webviews_survives_delayed_selection() {
    assert_eq!(std::env::var("JASONSHELL_SNIP_ESCAPE_NATIVE_TESTS").as_deref(),Ok("1"));
    let approved=std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode");
    if let Ok(id)=std::env::var("JASONSHELL_SNIP_ESCAPE_WEBVIEW_CHILD") {
        let parts:Vec<_>=id.strip_prefix("snip-escape-webview-").unwrap().split('-').collect();
        assert_eq!(parts.len(),2);assert!(parts.iter().all(|p|!p.is_empty()&&p.bytes().all(|b|b.is_ascii_digit())));
        let profile=approved.join(&id);
        assert!(!std::fs::symlink_metadata(&profile).unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read(profile.join("parent-owner")).unwrap(),id.as_bytes());
        run_escape_webview_lifecycle(profile);return;
    }
    assert!(!std::fs::symlink_metadata(&approved).unwrap().file_type().is_symlink());
    let id=format!("snip-escape-webview-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    let path=approved.join(&id);std::fs::create_dir(&path).unwrap();
    let profile=OwnedWebviewProfile(path.clone());
    std::fs::write(path.join("parent-owner"),id.as_bytes()).unwrap();
    let mut child=OwnedChild(Command::new(std::env::current_exe().unwrap())
        .args(["--ignored","--exact",WEBVIEW_CASE,"--test-threads=1","--nocapture"])
        .env("JASONSHELL_SNIP_ESCAPE_WEBVIEW_CHILD",id)
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap());
    let out=child.0.stdout.take().unwrap();let err=child.0.stderr.take().unwrap();
    // Drain both streams concurrently; native diagnostics cannot block child exit.
    let stdout=thread::spawn(move||{use std::io::Read;let mut bytes=Vec::new();out.take(65536).read_to_end(&mut bytes).unwrap();bytes});
    let stderr=thread::spawn(move||{use std::io::Read;let mut bytes=Vec::new();err.take(65536).read_to_end(&mut bytes).unwrap();bytes});
    let deadline=Instant::now()+Duration::from_secs(50);let mut timed_out=false;
    while child.0.try_wait().unwrap().is_none(){
        if Instant::now()>=deadline{timed_out=true;child.0.kill().unwrap();break;}
        thread::sleep(Duration::from_millis(20));
    }
    let status=child.0.wait().unwrap();drop(child);
    let out=stdout.join().unwrap();let err=stderr.join().unwrap();
    drop(profile);assert!(!path.exists(),"exact owned WebView profile must be removed after child exit");
    println!("{}\n{}",String::from_utf8_lossy(&out),String::from_utf8_lossy(&err));
    metadata("SNIP_ESCAPE_WEBVIEW_OWNED_PROFILE_REMOVED=1");
    assert!(!timed_out,"bounded lifecycle child timed out");assert!(status.success(),"actual native/WebView lifecycle failed");
    assert!(String::from_utf8_lossy(&out).contains("SNIP_ESCAPE_WEBVIEW_LIFECYCLE_COMPLETED=1"),"no zero-stage success");
}

fn run_escape_webview_lifecycle(profile:std::path::PathBuf) {
    let f=Fixture::new();
    let listener=Listener(crate::snipping::escape::install(Arc::downgrade(&f.runtime)).expect("install listener BEFORE any WebView/Tauri startup"));
    metadata("SNIP_ESCAPE_LISTENER_BEFORE_WEBVIEWS=1");
    let result=Arc::new(Mutex::new(None));let results=result.clone();
    let done=Arc::new(std::sync::atomic::AtomicBool::new(false));let completed=done.clone();
    let app=tauri::Builder::default().any_thread()
        .register_uri_scheme_protocol("escape-lifecycle",|_,_|tauri::http::Response::builder().header("Content-Type","text/html")
            .body(b"<!doctype html><html><head><title>plain owned lifecycle fixture</title></head><body></body></html>".to_vec()).unwrap())
        .setup(move|app|{
            let (ready,loaded)=mpsc::channel();
            for label in ["escape-lifecycle-a","escape-lifecycle-b"] {
                let ready=ready.clone();
                WebviewWindowBuilder::new(app,label,WebviewUrl::CustomProtocol("escape-lifecycle://localhost/".parse()?))
                    .data_directory(profile.join(label)).visible(false).focused(false).skip_taskbar(true).inner_size(64.0,64.0)
                    .on_page_load(|window,payload|{if payload.event()==tauri::webview::PageLoadEvent::Finished {
                        let _=window.eval("(()=>{for(let n=0;n<10000;n++){const x=document.createElement('span');x.textContent='synthetic';document.body.append(x);x.remove()}document.title='escape-lifecycle-ready'})()");
                    }})
                    .on_document_title_changed(move|_,title|{if title=="escape-lifecycle-ready"{let _=ready.send(label);}}).build()?;
            }
            let handle=app.handle().clone();let watchdog=handle.clone();let watchdog_done=completed.clone();
            thread::spawn(move||{
                let success=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{
                    let mut labels=std::collections::BTreeSet::new();
                    while labels.len()<2 {labels.insert(loaded.recv_timeout(Duration::from_secs(10)).expect("both real WebViews must finish synthetic workload"));}
                    // Listener has already survived WebView2 startup/workload;
                    // selection starts only after an additional idle interval.
                    thread::sleep(Duration::from_secs(3));
                    let t=f.accepted();escape_overlay(&f,&t);
                    run_native_pulse(f,listener);
                    metadata("SNIP_ESCAPE_WEBVIEW_LIFECYCLE_COMPLETED=1");
                })).is_ok();
                *results.lock().unwrap()=Some(success);completed.store(true,EscapeOrdering::Release);handle.exit(0);
            });
            thread::spawn(move||{
                let end=Instant::now()+Duration::from_secs(35);
                while !watchdog_done.load(EscapeOrdering::Acquire)&&Instant::now()<end {thread::sleep(Duration::from_millis(25));}
                if !watchdog_done.load(EscapeOrdering::Acquire){watchdog.exit(3);}
            });
            Ok(())
        }).build(tauri::generate_context!()).unwrap();
    let code=app.run_return(|_,_|{});done.store(true,EscapeOrdering::Release);
    assert_eq!(code,0,"lifecycle watchdog/exit status");
    assert_eq!(result.lock().unwrap().take(),Some(true),"native lifecycle assertions must execute and pass");
}
