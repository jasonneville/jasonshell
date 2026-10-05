//! Actual production installer + owned synthetic disk; loader is NOT ONNX inference.
#[path = "../src/speech_model_install.rs"] mod speech_model_install;
use speech_model_install::{install_directory,resolve_installed_model,InstallLimits};
use std::{fs,path::{Path,PathBuf},sync::atomic::{AtomicU64,Ordering}};
const FILES:[&str;3]=["encoder-model.int8.onnx","decoder_joint-model.int8.onnx","vocab.txt"];
const BYTES:[&[u8];3]=[b"synthetic encoder",b"synthetic decoder",b"synthetic vocab"];
static NEXT:AtomicU64=AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new()->Self {
        let root=PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode")
            .join(format!("speech-folder-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();Self(root)
    }
    fn source(&self)->PathBuf {let p=self.0.join("source");fs::create_dir(&p).unwrap();for (name,bytes) in FILES.iter().zip(BYTES) {fs::write(p.join(name),bytes).unwrap();}p}
    fn data(&self)->PathBuf {self.0.join("shared-app-data")}
    fn no_staging(&self) {let root=self.data().join("speech-models");if root.exists() {assert!(fs::read_dir(root).unwrap().all(|e|!e.unwrap().file_name().to_string_lossy().starts_with("staging-")));}}
}
impl Drop for Fixture {fn drop(&mut self) {let _=fs::remove_dir_all(&self.0);}}
fn readback(p:&Path) {for (name,bytes) in FILES.iter().zip(BYTES) {assert_eq!(fs::read(p.join(name)).unwrap(),bytes);}}
#[test]
fn folder_copies_only_three_root_files_and_remains_resolvable_without_source() {
    let f=Fixture::new();let source=f.source();fs::write(source.join("config.json"),b"ignore").unwrap();
    fs::write(source.join("nemo128.onnx"),b"ignore extra").unwrap();
    fs::create_dir(source.join("nested")).unwrap();fs::write(source.join("nested/vocab.txt"),b"ignore nested").unwrap();
    let mut validated=false;
    let installed=install_directory(&source,&f.data(),InstallLimits::default(),|stage| {
        assert!(stage.starts_with(f.data()));assert_ne!(stage,source);readback(stage);
        assert_eq!(fs::read_dir(stage).unwrap().count(),3);assert!(resolve_installed_model(&f.data()).is_none());
        validated=true;Ok(())
    }).unwrap();assert!(validated);readback(&source);assert!(source.join("config.json").exists());
    assert_eq!(resolve_installed_model(&f.data()),Some(installed.clone()));
    fs::remove_dir_all(source).unwrap();readback(&resolve_installed_model(&f.data()).unwrap());
    assert_eq!(fs::read_dir(installed).unwrap().count(),3);f.no_staging();
}
#[test]
fn invalid_missing_empty_directory_required_and_nested_only_fail_before_validation() {
    for variant in 0..5 {let f=Fixture::new();let source=f.source();
        match variant {0=>{fs::remove_file(source.join(FILES[0])).unwrap();},1=>{fs::write(source.join(FILES[1]),b"").unwrap();},
            2=>{fs::remove_file(source.join(FILES[2])).unwrap();fs::create_dir(source.join(FILES[2])).unwrap();},
            3=>{fs::create_dir(source.join("nested")).unwrap();for name in FILES {fs::rename(source.join(name),source.join("nested").join(name)).unwrap();}},
            _=>{fs::remove_dir_all(&source).unwrap();fs::write(&source,b"not a directory").unwrap();}}
        let mut loaded=false;assert!(install_directory(&source,&f.data(),InstallLimits::default(),|_|{loaded=true;Ok(())}).is_err());
        assert!(!loaded);assert!(resolve_installed_model(&f.data()).is_none());f.no_staging();
    }
}
#[test]
fn required_file_and_aggregate_limits_are_enforced_before_loader() {
    for limits in [InstallLimits{max_file_bytes:1,..InstallLimits::default()},InstallLimits{max_total_bytes:2,..InstallLimits::default()}] {
        let f=Fixture::new();let source=f.source();let mut loaded=false;
        assert!(install_directory(&source,&f.data(),limits,|_|{loaded=true;Ok(())}).is_err());assert!(!loaded);readback(&source);f.no_staging();
    }
}
#[test]
fn exact_required_byte_caps_allow_import_even_with_large_ignored_extra() {
    let f=Fixture::new();let source=f.source();fs::write(source.join("nemo128.onnx"),vec![7;8192]).unwrap();
    let limits=InstallLimits{max_file_bytes:BYTES.iter().map(|b|b.len() as u64).max().unwrap(),max_total_bytes:BYTES.iter().map(|b|b.len() as u64).sum(),..InstallLimits::default()};
    let installed=install_directory(&source,&f.data(),limits,|p|{readback(p);Ok(())}).unwrap();readback(&installed);f.no_staging();
}
#[test]
fn failed_validation_keeps_prior_generation_selected_and_cleans_staging() {
    let f=Fixture::new();let source=f.source();let old=install_directory(&source,&f.data(),InstallLimits::default(),|_|Ok(())).unwrap();
    let mut calls=0;let failure=install_directory(&source,&f.data(),InstallLimits::default(),|stage| {
        calls+=1;assert_ne!(stage,old);readback(stage);assert_eq!(resolve_installed_model(&f.data()),Some(old.clone()));Err("synthetic loader failure".into())
    });assert!(failure.is_err());assert_eq!(calls,1);assert_eq!(resolve_installed_model(&f.data()),Some(old));readback(&source);f.no_staging();
}
#[cfg(windows)]
#[test]
fn directory_junction_is_rejected_without_following_or_mutating_target() {
    let f=Fixture::new();let source=f.source();let link=f.0.join("junction");
    use std::os::windows::process::CommandExt;
    // cmd's built-in mklink rejects forward-slash path segments as switches.
    // Render ONLY these exclusively owned fixture paths as quoted native paths.
    let command=format!("mklink /J \"{}\" \"{}\"",link.to_string_lossy().replace('/',"\\"),source.to_string_lossy().replace('/',"\\"));
    let status=std::process::Command::new("cmd").args(["/D","/C"]).raw_arg(command).output().unwrap();
    assert!(status.status.success(),"junction fixture creation must not be silently skipped: {}",String::from_utf8_lossy(&status.stderr));
    use std::os::windows::fs::MetadataExt;
    assert_ne!(std::fs::symlink_metadata(&link).unwrap().file_attributes() & 0x400,0,"fixture must be an actual Windows reparse point");
    let mut loaded=false;let result=install_directory(&link,&f.data(),InstallLimits::default(),|_|{loaded=true;Ok(())});
    fs::remove_dir(&link).unwrap();assert!(result.is_err());assert!(!loaded);readback(&source);f.no_staging();
}
#[cfg(windows)]
#[test]
fn required_file_symlink_is_rejected_without_following_target() {
    let f=Fixture::new();let source=f.source();let target=f.0.join("owned-target");fs::write(&target,BYTES[0]).unwrap();
    fs::remove_file(source.join(FILES[0])).unwrap();
    std::os::windows::fs::symlink_file(&target,source.join(FILES[0])).expect("symlink fixture privilege needed; never silently skip security test");
    let mut loaded=false;assert!(install_directory(&source,&f.data(),InstallLimits::default(),|_|{loaded=true;Ok(())}).is_err());
    assert!(!loaded);assert_eq!(fs::read(target).unwrap(),BYTES[0]);f.no_staging();
}
