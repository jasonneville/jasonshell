//! Opt-in actual WebView2 -> Tauri IPC ACL test, harmless terminals ONLY.
//! Hidden/nonactivating windows; no production modules/handlers/main are loaded.
#![cfg(windows)]
use std::{collections::BTreeMap,io::{Read,Write},net::TcpListener,path::PathBuf,process::{Command,Stdio},
    sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}},thread,time::{Duration,Instant}};
use tauri::{Manager,WebviewUrl,WebviewWindow,WebviewWindowBuilder,webview::PageLoadEvent};
#[derive(Default)] struct Markers(Mutex<Vec<(String,String)>>);
fn marker(window:&WebviewWindow,state:&Markers,command:&str)->String {
    state.0.lock().unwrap().push((window.label().into(),command.into()));
    format!("harmless:{command}:{}",window.label())
}
// Deliberately share registered production NAMES, NOT production implementation.
// No activation/file/menu/capture method is reachable from these terminals.
#[tauri::command] fn list_open_task_windows(window:WebviewWindow,state:tauri::State<Markers>)->String {marker(&window,&state,"list_open_task_windows")}
#[tauri::command] fn activate_task_window(window:WebviewWindow,state:tauri::State<Markers>)->String {marker(&window,&state,"activate_task_window")}
#[tauri::command] fn list_pinned_taskbar_apps(window:WebviewWindow,state:tauri::State<Markers>)->String {marker(&window,&state,"list_pinned_taskbar_apps")}
#[tauri::command] fn show_top_bar_pin_context_menu(window:WebviewWindow,state:tauri::State<Markers>)->String {marker(&window,&state,"show_top_bar_pin_context_menu")}
#[tauri::command] fn start_snip(window:WebviewWindow,state:tauri::State<Markers>)->String {marker(&window,&state,"start_snip")}
#[tauri::command] fn copy_snip(window:WebviewWindow,state:tauri::State<Markers>)->String {marker(&window,&state,"copy_snip")}
// Folder import transport ONLY: never opens picker or imports a real model.
#[tauri::command] fn import_speech_model_folder(window:WebviewWindow,state:tauri::State<Markers>)->String {marker(&window,&state,"import_speech_model_folder")}
type Results=Arc<Mutex<BTreeMap<String,serde_json::Value>>>;
struct OwnedDirectory(PathBuf);
impl Drop for OwnedDirectory {fn drop(&mut self) {for _ in 0..15 {
    if std::fs::remove_dir_all(&self.0).is_ok() || !self.0.exists() {return;}
    thread::sleep(Duration::from_millis(200));
}}}
fn isolated_child(force_timeout:bool,folder:bool)->Result<(), &'static str> {
    // Parent allocates/guards exact directory BEFORE spawn. A killed child need
    // not print stdout or run Drop for the parent to know what it must remove.
    let id=format!("shell-acl-ipc-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    let path=PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode").join(&id);
    std::fs::create_dir(&path).unwrap();let directory=OwnedDirectory(path.clone());
    std::fs::write(path.join("parent-owner"),id.as_bytes()).unwrap();
    let mut child=Command::new(std::env::current_exe().unwrap())
        .args(["--ignored","--exact","native_crosssurface_ipc_reaches_only_authorized_harmless_terminals","--test-threads=1","--nocapture"])
        .env("JASONSHELL_ACL_NATIVE_CHILD","1").env("JASONSHELL_ACL_NATIVE_PROFILE_ID",&id)
        .env("JASONSHELL_ACL_NATIVE_FORCE_TIMEOUT",if force_timeout {"1"} else {"0"})
        .env("JASONSHELL_ACL_NATIVE_FOLDER",if folder {"1"} else {"0"})
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let mut deadline=Instant::now()+Duration::from_secs(if force_timeout {5} else {40});
    let mut fault_ready=false;let mut timed_out=false;
    loop {if child.try_wait().unwrap().is_some() {break;}
        if force_timeout && !fault_ready && path.join("fault-ready").is_file() {fault_ready=true;deadline=Instant::now()+Duration::from_millis(50);}
        if Instant::now()>=deadline {
            timed_out=true;child.kill().expect("terminate exact owned native IPC child");
            let exit_deadline=Instant::now()+Duration::from_secs(2);
            while child.try_wait().unwrap().is_none() && Instant::now()<exit_deadline {thread::sleep(Duration::from_millis(10));}
            assert!(child.try_wait().unwrap().is_some(),"owned child did not exit within termination bound");
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let output=child.wait_with_output().unwrap();
    // Success, timeout, failed assertions, malformed stdout: same cleanup guard,
    // with removal BEFORE any result/metadata failure is reported to the caller.
    drop(directory);
    assert!(!path.exists(),"owned synthetic WebView profiles were not removed AFTER child exit");
    if timed_out {
        if force_timeout {assert!(fault_ready,"fault path must initialize before timeout; no vacuous kill proof");}
        assert!(!output.status.success(),"forced termination must produce non-success child status");
        eprintln!("forced timeout: exact owned child exit verified; parent-owned profile removed BEFORE timeout error");
        return Err("owned native IPC child deadline exceeded");
    }
    assert!(output.stdout.len()+output.stderr.len()<16384,"bounded owned fixture metadata exceeded");
    let stdout=String::from_utf8(output.stdout).unwrap();
    let reported=stdout.lines().find_map(|line|line.split_once("ACL_PROFILE_ID=").map(|(_,id)|id.trim())).expect("child did not report supplied fixture ID");
    assert_eq!(reported,id,"child must use exact parent-allocated profile, not arbitrary output path");
    assert!(output.status.success(),"actual native IPC child failed: {stdout}\n{}",String::from_utf8_lossy(&output.stderr));
    eprintln!("actual isolated WebView IPC: folder={folder};post-exit owned-profile cleanup verified\n{}",String::from_utf8_lossy(&output.stderr));
    Ok(())
}
#[test]
#[ignore = "explicit JASONSHELL_ACL_NATIVE_TESTS=1; synthetic child forced-timeout cleanup only"]
fn native_ipc_forced_timeout_cleans_exact_parent_owned_profile() {
    assert_eq!(std::env::var("JASONSHELL_ACL_NATIVE_TESTS").as_deref(),Ok("1"));
    assert_eq!(isolated_child(true,false),Err("owned native IPC child deadline exceeded"));
}
#[test]
#[ignore = "explicit JASONSHELL_ACL_NATIVE_TESTS=1; harmless folder command transport, NOT picker/model inference"]
fn native_folder_import_settings_transport_preserves_denials() {
    assert_eq!(std::env::var("JASONSHELL_ACL_NATIVE_TESTS").as_deref(),Ok("1"));
    isolated_child(false,true).expect("folder native transport outcome AFTER cleanup");
}
#[test]
#[ignore = "requires explicit JASONSHELL_ACL_NATIVE_TESTS=1; hidden synthetic WebView IPC only"]
fn native_crosssurface_ipc_reaches_only_authorized_harmless_terminals() {
    assert_eq!(std::env::var("JASONSHELL_ACL_NATIVE_TESTS").as_deref(),Ok("1"),"explicit native test opt-in required");
    if std::env::var("JASONSHELL_ACL_NATIVE_CHILD").as_deref()!=Ok("1") {isolated_child(false,false).expect("native IPC child outcome after cleanup");return;}
    let folder=std::env::var("JASONSHELL_ACL_NATIVE_FOLDER").as_deref()==Ok("1");
    let expected_surfaces=if folder {4} else {5};
    let approved=PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode");
    let id=std::env::var("JASONSHELL_ACL_NATIVE_PROFILE_ID").expect("parent-provided fixture ID");
    let parts:Vec<_>=id.strip_prefix("shell-acl-ipc-").unwrap().split('-').collect();
    assert!(parts.len()==2&&parts.iter().all(|s|!s.is_empty()&&s.bytes().all(|b|b.is_ascii_digit())),"bounded fixture ID, never an arbitrary path");
    let path=approved.join(&id);assert!(!std::fs::symlink_metadata(&path).unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read(path.join("parent-owner")).unwrap(),id.as_bytes());
    let directory=OwnedDirectory(path);
    println!("ACL_PROFILE_ID={}",directory.0.file_name().unwrap().to_str().unwrap());
    if std::env::var("JASONSHELL_ACL_NATIVE_FORCE_TIMEOUT").as_deref()==Ok("1") {
        // Test-only dead child BEFORE any Tauri/window/server initialization.
        // Parent kill bypasses this directory's Drop exactly as the review defect.
        std::fs::write(directory.0.join("fault-ready"),b"synthetic forced-timeout child").unwrap();
        loop {thread::sleep(Duration::from_secs(1));}
    }
    // Own loopback-only static document is REMOTE to app ACL, not an external site.
    let server=TcpListener::bind("127.0.0.1:0").unwrap();server.set_nonblocking(true).unwrap();
    let remote=format!("http://{}/",server.local_addr().unwrap());
    let stop=Arc::new(AtomicBool::new(false));let stopped=stop.clone();
    let serving=thread::spawn(move|| {
        while !stopped.load(Ordering::Acquire) {
            match server.accept() {Ok((mut socket,_))=> {
                let _=socket.set_read_timeout(Some(Duration::from_millis(500)));let mut request=[0u8;2048];let _=socket.read(&mut request);
                let body="<!doctype html><html><head><title>harmless ACL fixture</title></head><body></body></html>";
                let response=format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());let _=socket.write_all(response.as_bytes());
            },Err(_)=>thread::sleep(Duration::from_millis(10))}
        }
    });
    let results:Results=Arc::new(Mutex::new(BTreeMap::new()));let reports=results.clone();
    let profiles=directory.0.clone();let finished=Arc::new(AtomicBool::new(false));let watchdog_done=finished.clone();
    // Production generated ACL/capabilities are loaded UNCHANGED by this macro.
    // No add_capability/allow_command/test-report command bypass is used.
    let context=tauri::generate_context!();
    let app=tauri::Builder::default().any_thread().manage(Markers::default())
        .register_uri_scheme_protocol("acl-fixture",|_,_|tauri::http::Response::builder()
            .header("Content-Type","text/html").body(b"<!doctype html><html><head><title>harmless ACL fixture</title></head><body></body></html>".to_vec()).unwrap())
        .invoke_handler(tauri::generate_handler![list_open_task_windows,activate_task_window,list_pinned_taskbar_apps,show_top_bar_pin_context_menu,start_snip,copy_snip,import_speech_model_folder])
        .setup(move|app| {
            let cases=if folder {vec![
                ("settings-panel",false,vec![("import_speech_model_folder",true),("start_snip",false)]),
                ("acl-unknown",false,vec![("import_speech_model_folder",false)]),
                ("control-plane",true,vec![("import_speech_model_folder",false)]),
                ("snip-preview-7",false,vec![("import_speech_model_folder",false)]),
            ]} else {vec![
                ("top-bar",false,vec![("list_pinned_taskbar_apps",true),("show_top_bar_pin_context_menu",true)]),
                ("bottom-bar",false,vec![("list_open_task_windows",true),("activate_task_window",true),("start_snip",false)]),
                ("acl-unknown",false,vec![("list_open_task_windows",false),("start_snip",false)]),
                ("quick-launch-panel",true,vec![("list_open_task_windows",false),("start_snip",false)]),
                ("snip-preview-7",true,vec![("copy_snip",false),("start_snip",false)]),
            ]};
            for (label,is_remote,cases) in cases {
                let commands=serde_json::to_string(&cases)?;
                let script=format!(r#"(async()=>{{const results=[];const bridge=typeof window.__TAURI_INTERNALS__?.invoke==='function';for(const [command,expected] of {commands}){{try{{const marker=await window.__TAURI_INTERNALS__.invoke(command);results.push({{command,expected,bridge,allowed:true,marker}})}}catch(error){{results.push({{command,expected,bridge,allowed:false,aclDenied:/not allowed|not permitted|permission|forbidden/i.test(String(error))}})}}}}document.title='acl-result:'+JSON.stringify(results)}})();"#);
                let state=reports.clone();let handle=app.handle().clone();
                let url=if is_remote {WebviewUrl::External(remote.parse()?)} else {WebviewUrl::CustomProtocol("acl-fixture://localhost/".parse()?)};
                WebviewWindowBuilder::new(app,label,url).visible(false).focused(false).skip_taskbar(true)
                    .data_directory(profiles.join(label))
                    .on_page_load(move|window,payload| {if payload.event()==PageLoadEvent::Finished {let _=window.eval(&script);}})
                    .on_document_title_changed(move|window,title| {
                        if let Some(json)=title.strip_prefix("acl-result:").filter(|s|s.len()<=4096) {
                            if let Ok(value)=serde_json::from_str(json) {
                                let mut state=state.lock().unwrap();state.insert(window.label().into(),value);
                                if state.len()==expected_surfaces {handle.exit(0);}
                            }
                        }
                    }).build()?;
            }
            let handle=app.handle().clone();thread::spawn(move|| {
                let end=Instant::now()+Duration::from_secs(25);
                while !watchdog_done.load(Ordering::Acquire) && Instant::now()<end {thread::sleep(Duration::from_millis(50));}
                if !watchdog_done.load(Ordering::Acquire) {handle.exit(3);}
            });
            Ok(())
        }).build(context).unwrap();
    let handle=app.handle().clone();
    let code=app.run_return(|_,_|{});
    finished.store(true,Ordering::Release);stop.store(true,Ordering::Release);serving.join().unwrap();
    let reports=results.lock().unwrap();assert_eq!(code,0,"native IPC watchdog/exit failure; completed labels={:?}",reports.keys().collect::<Vec<_>>());
    assert_eq!(reports.len(),expected_surfaces,"must execute every local/unknown/remote WebView, not zero-test GREEN");
    let mut assertions=0;
    for (label,values) in reports.iter() {for result in values.as_array().expect("owned renderer result array") {
        // Host oracle is independent of renderer's reported expected/allowed bits.
        let command=result["command"].as_str().unwrap();
        let permitted=if folder {label=="settings-panel"&&command=="import_speech_model_folder"} else {matches!((label.as_str(),command),
            ("top-bar","list_pinned_taskbar_apps"|"show_top_bar_pin_context_menu")|
            ("bottom-bar","list_open_task_windows"|"activate_task_window"))};
        assert_eq!(result["allowed"],permitted,"native IPC {label} -> {command}");
        assert_eq!(result["bridge"],true,"{label}: denied must not mean missing JS IPC bridge");
        if permitted {assert_eq!(result["marker"],format!("harmless:{command}:{label}"));}
        else {assert_eq!(result["aclDenied"],true,"{label} -> {command}: expected actual ACL rejection, not generic JS failure");}
        assertions+=1;
    }}
    assert_eq!(assertions,if folder {5} else {11});
    let calls=handle.state::<Markers>().0.lock().unwrap().clone();
    assert_eq!(calls.len(),if folder {1} else {4},"denied IPC must never enter harmless terminal");
    assert!(!calls.iter().any(|(_,command)|command=="start_snip"||command=="copy_snip"));
    eprintln!("actual WebView IPC:folder={folder};{expected_surfaces} hidden surfaces;{} permitted markers;{} denials;snipping handlerCalls=0;no picker/install/activation/menu/file/capture handler run",calls.len(),assertions-calls.len());
    drop(handle);drop(directory);
    // Parent verifies final cleanup after this isolated test process exits.
}
