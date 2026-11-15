//! Execute the verbatim vendored callback under a no-OS/no-COM symbol facade.
//! This checks upstream removal ordering, not native HWND/COM crash reproduction.
use std::{fs,path::PathBuf,process::{Command,Stdio},thread,time::{Duration,Instant}};
fn bounded(program:&str,args:&[&std::ffi::OsStr],log:&std::path::Path)->std::process::Output{
    let file=fs::File::create(log).unwrap();
    let mut child=Command::new(program).args(args).stdout(Stdio::from(file.try_clone().unwrap())).stderr(Stdio::from(file)).spawn().unwrap();
    let deadline=Instant::now()+Duration::from_secs(60);
    let status=loop{if let Some(status)=child.try_wait().unwrap(){break status;}
        if Instant::now()>=deadline{let _=child.kill();let _=child.wait();panic!("bounded callback facade child timed out; log {}",log.display());}
        thread::sleep(Duration::from_millis(20));};
    std::process::Output{status,stdout:fs::read(log).unwrap(),stderr:Vec::new()}
}
#[test]
fn actual_vendored_parent_callback_retires_before_controller_release(){
    let root=PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source=fs::read_to_string(root.join("vendor/wry/src/webview2/mod.rs")).expect("approved vendored dependency source must exist");
    let signature="unsafe extern \"system\" fn parent_subclass_proc(";
    assert_eq!(source.matches(signature).count(),1,"callback extraction must be unambiguous");
    let start=source.find(signature).unwrap();let brace=start+source[start..].find('{').unwrap();
    let mut depth=0;let mut end=None;
    for (index,byte) in source.as_bytes().iter().enumerate().skip(brace){
        if *byte==b'{'{depth+=1;}else if *byte==b'}'{depth-=1;if depth==0{end=Some(index+1);break;}}
    }
    let callback=&source[start..end.expect("complete callback body")];
    let facade=include_str!("fixtures/wry_parent_subclass_facade.rs");
    assert_eq!(facade.matches("/* ACTUAL_CALLBACK */").count(),1);
    // No transformations or replacements inside the actual callback body.
    let harness=facade.replace("/* ACTUAL_CALLBACK */",callback);
    let temporary=PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode").join(format!("wry-callback-retirement-{}",std::process::id()));
    fs::create_dir_all(&temporary).unwrap();
    let input=temporary.join("callback.rs");let executable=temporary.join("callback-tests.exe");fs::write(&input,harness).unwrap();
    let compile=bounded("rustc",&["--edition=2021".as_ref(),"--test".as_ref(),input.as_os_str(),"-o".as_ref(),executable.as_os_str()],&temporary.join("compile.log"));
    assert!(compile.status.success(),"actual callback facade compile failed: {}",String::from_utf8_lossy(&compile.stdout));
    let run=bounded(executable.to_str().unwrap(),&["--test-threads=1".as_ref(),"--nocapture".as_ref()],&temporary.join("run.log"));
    let output=String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(),"actual callback retirement regression: {output}; artifacts {}",temporary.display());
    assert!(output.contains("test result: ok. 2 passed; 0 failed; 0 ignored;"),"both actual callback cases must execute: {output}");
}
