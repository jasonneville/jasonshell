#![cfg(all(test, windows))]
#![allow(dead_code)]

use std::{path::PathBuf, rc::Rc};
use windows::{core::{w, PCWSTR}, Win32::{Foundation::{HWND, POINTL}, System::{Com::{IDataObject, FORMATETC, DVASPECT_CONTENT, TYMED_HGLOBAL}, Ole::{OleInitialize, OleUninitialize, IDropTarget, CF_HDROP, DROPEFFECT_COPY, DROPEFFECT_NONE}, SystemServices::MODIFIERKEYS_FLAGS}, UI::{Shell::SHCreateDataObject, WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_CHILD, WS_OVERLAPPED, WINDOW_EX_STYLE, WINDOW_STYLE}}}};

pub enum DragDropEvent {
    Enter { paths: Vec<PathBuf>, position: (f64, f64) },
    Over { position: (f64, f64) },
    Drop { paths: Vec<PathBuf>, position: (f64, f64) },
    Leave,
}

#[path = "../../../src-tauri/vendor/wry/src/webview2/drag_drop.rs"]
mod drag_drop;

struct Ole;
impl Drop for Ole {
    fn drop(&mut self) { unsafe { OleUninitialize() } }
}
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) { unsafe { DestroyWindow(self.0).unwrap() } }
}
fn window(parent: Option<HWND>, style: WINDOW_STYLE) -> Window {
    Window(unsafe { CreateWindowExW(WINDOW_EX_STYLE(0), w!("STATIC"), PCWSTR::null(), style, 0, 0, 100, 100, parent, None, None, None) }.unwrap())
}

fn scenario(test: impl FnOnce(HWND, &IDataObject, IDropTarget)) {
    unsafe { OleInitialize(None).unwrap() };
    let _ole = Ole;
    let owner = window(None, WS_OVERLAPPED);
    let child = window(Some(owner.0), WS_CHILD);
    let object: IDataObject = unsafe { SHCreateDataObject(None, None, None::<&IDataObject>) }.expect("create empty Shell data object");
    let format = FORMATETC { cfFormat: CF_HDROP.0, dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32, ..Default::default() };
    assert!(unsafe { object.GetData(&format) }.is_err(), "fixture must not offer CF_HDROP");
    let target: IDropTarget = drag_drop::DragDropTarget::new(child.0, Rc::new(|_| panic!("must not publish external events"))).into();
    test(owner.0, &object, target);
}

#[test]
fn exact_local_identity_negotiates_copy_without_hdrop() {
    scenario(|owner, object, target| {
        let pt = POINTL { x: 12, y: 16 };
        let keys = MODIFIERKEYS_FLAGS(0);
        let ((enter, over), _) = drag_drop::observe_local_file_drop(owner, object, || unsafe {
            let mut enter = DROPEFFECT_COPY;
            target.DragEnter(object, keys, pt, &mut enter).unwrap();
            let mut over = DROPEFFECT_COPY;
            target.DragOver(keys, pt, &mut over).unwrap();
            (enter, over)
        });
        assert_eq!((enter.0, over.0), (DROPEFFECT_COPY.0, DROPEFFECT_COPY.0));
    });
}

#[test]
fn unknown_unsupported_object_clears_offered_copy() {
    scenario(|_, object, target| unsafe {
        let mut effect = DROPEFFECT_COPY;
        target.DragEnter(object, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut effect).unwrap();
        assert_eq!(effect.0, DROPEFFECT_NONE.0);
    });
}

#[test]
fn exact_local_drop_returns_copy_and_position() {
    scenario(|owner, object, target| {
        let pt = POINTL { x: 12, y: 16 };
        // OLE's effect is in/out. Seed COPY plus MOVE to verify the callback
        // selects COPY, rather than leaving an allowed-effects mask untouched.
        let ((drop_effect, after_drop), position) = drag_drop::observe_local_file_drop(owner, object, || unsafe {
            let mut effect = DROPEFFECT_COPY;
            target.DragEnter(object, MODIFIERKEYS_FLAGS(0), pt, &mut effect).unwrap();
            effect = windows::Win32::System::Ole::DROPEFFECT(3);
            target.Drop(object, MODIFIERKEYS_FLAGS(0), pt, &mut effect).unwrap();
            let mut after = DROPEFFECT_COPY;
            target.DragOver(MODIFIERKEYS_FLAGS(0), pt, &mut after).unwrap();
            (effect, after)
        });
        assert!(position.is_some(), "exact object must yield child-client receipt position");
        assert_eq!((drop_effect, after_drop), (DROPEFFECT_COPY, DROPEFFECT_NONE));
    });
}

#[test]
fn local_without_copy_offer_cannot_capture() {
    scenario(|owner, object, target| {
        let keys = MODIFIERKEYS_FLAGS(0);
        let pt = POINTL::default();
        let (effects, position) = drag_drop::observe_local_file_drop(owner, object, || unsafe {
            let mut enter = windows::Win32::System::Ole::DROPEFFECT_MOVE;
            target.DragEnter(object, keys, pt, &mut enter).unwrap();
            let mut drop = windows::Win32::System::Ole::DROPEFFECT_MOVE;
            target.Drop(object, keys, pt, &mut drop).unwrap();
            (enter, drop)
        });
        assert!(position.is_none());
        assert_eq!(effects, (DROPEFFECT_NONE, DROPEFFECT_NONE));
    });
}

#[test]
fn leave_clears_cached_local_copy() {
    scenario(|owner, object, target| {
        drag_drop::observe_local_file_drop(owner, object, || unsafe {
            let keys = MODIFIERKEYS_FLAGS(0);
            let pt = POINTL::default();
            let mut effect = DROPEFFECT_COPY;
            target.DragEnter(object, keys, pt, &mut effect).unwrap();
            target.DragOver(keys, pt, &mut effect).unwrap();
            assert_eq!(effect, DROPEFFECT_COPY);
            target.DragLeave().unwrap();
            target.DragOver(keys, pt, &mut effect).unwrap();
            assert_eq!(effect, DROPEFFECT_NONE);
        });
    });
}

#[test]
fn same_parent_shell_object_offers_hdrop_for_real_files() {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::{System::Ole::ReleaseStgMedium, UI::Shell::{ILCreateFromPathW, ILFindLastID, ILFree, Common::ITEMIDLIST}};
    struct Pidl(*mut ITEMIDLIST);
    impl Drop for Pidl {
        fn drop(&mut self) { unsafe { ILFree(Some(self.0)) } }
    }
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) { std::fs::remove_dir_all(&self.0).unwrap(); }
    }
    let root = std::env::temp_dir().join(format!("wry-drop-negotiation-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let root = Temp(root);
    let pidl = |path: &std::path::Path| {
        let wide: Vec<_> = path.as_os_str().encode_wide().chain([0]).collect();
        let id = unsafe { ILCreateFromPathW(PCWSTR(wide.as_ptr())) };
        assert!(!id.is_null());
        Pidl(id)
    };
    unsafe { OleInitialize(None).unwrap() };
    let _ole = Ole;
    let parent = pidl(&root.0);
    let items = ["one.txt", "two.txt"].map(|name| {
        let path = root.0.join(name);
        std::fs::write(&path, name).unwrap();
        pidl(&path)
    });
    let children = items.each_ref().map(|item| unsafe { ILFindLastID(item.0).cast_const() });
    let object: IDataObject = unsafe { SHCreateDataObject(Some(parent.0), Some(&children), None::<&IDataObject>) }.unwrap();
    let format = FORMATETC { cfFormat: CF_HDROP.0, dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32, ..Default::default() };
    let mut medium = unsafe { object.GetData(&format) }.expect("same-parent Shell object GetData(CF_HDROP)");
    unsafe { ReleaseStgMedium(&mut medium) };

    // A second Shell object can describe the exact same paths without owning the receipt.
    let incoming: IDataObject = unsafe { SHCreateDataObject(Some(parent.0), Some(&children), None::<&IDataObject>) }.unwrap();
    let owner = window(None, WS_OVERLAPPED);
    let child = window(Some(owner.0), WS_CHILD);
    let drops = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = drops.clone();
    let target: IDropTarget = drag_drop::DragDropTarget::new(child.0, Rc::new(move |event| {
        if let DragDropEvent::Drop { paths, .. } = event { observed.borrow_mut().push(paths); }
        true
    })).into();
    let (_, receipt) = drag_drop::observe_local_file_drop(owner.0, &object, || unsafe {
        let pt = POINTL::default();
        let keys = MODIFIERKEYS_FLAGS(0);
        let mut effect = windows::Win32::System::Ole::DROPEFFECT(3);
        target.DragEnter(&incoming, keys, pt, &mut effect).unwrap();
        assert_eq!(effect, DROPEFFECT_COPY);
        effect = windows::Win32::System::Ole::DROPEFFECT(3);
        target.Drop(&incoming, keys, pt, &mut effect).unwrap();
        assert_eq!(effect, DROPEFFECT_COPY);
    });
    assert!(receipt.is_none());
    let mut paths = drops.borrow_mut().pop().expect("external drop published");
    paths.sort();
    assert_eq!(paths, [root.0.join("one.txt"), root.0.join("two.txt")]);
    assert!(drops.borrow().is_empty());
}
