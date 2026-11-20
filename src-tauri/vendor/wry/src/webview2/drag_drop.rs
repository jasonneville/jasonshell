// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

// A silly implementation of file drop handling for Windows!

use crate::DragDropEvent;

use std::{
  cell::{Cell, RefCell},
  ffi::OsString,
  os::{raw::c_void, windows::ffi::OsStringExt},
  path::PathBuf,
  ptr,
  rc::{Rc, Weak},
};

use windows::{
  core::{implement, Interface, IUnknown, BOOL},
  Win32::{
    Foundation::{DRAGDROP_E_INVALIDHWND, HWND, LPARAM, POINT, POINTL},
    Graphics::Gdi::ScreenToClient,
    System::{
      Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL},
      Ole::{
        IDropTarget, IDropTarget_Impl, RegisterDragDrop, RevokeDragDrop, CF_HDROP, DROPEFFECT,
        DROPEFFECT_COPY, DROPEFFECT_NONE,
      },
      SystemServices::MODIFIERKEYS_FLAGS,
    },
    UI::{
      Shell::{DragFinish, DragQueryFileW, HDROP},
      WindowsAndMessaging::{EnumChildWindows, IsChild},
    },
  },
};

struct LocalDropObservation {
  owner: HWND,
  source: IUnknown,
  position: Cell<Option<(i32, i32)>>,
}

thread_local! {
  static LOCAL_DROP: RefCell<Option<Rc<LocalDropObservation>>> = const { RefCell::new(None) };
}

/// Observe a drop from this exact COM object into an owner descendant during a synchronous
/// drag on the current STA. Matching drops are not published as ordinary webview drops.
pub fn observe_local_file_drop<T>(owner: HWND, data: &IDataObject, drag: impl FnOnce() -> T) -> (T, Option<(i32, i32)>) {
  let Ok(source) = data.cast::<IUnknown>() else { return (drag(), None) };
  let scope = Rc::new(LocalDropObservation { owner, source, position: Cell::new(None) });
  if !LOCAL_DROP.with(|slot| {
    let mut slot = slot.borrow_mut();
    if slot.is_some() { return false; }
    *slot = Some(scope.clone());
    true
  }) {
    return (drag(), None);
  }
  struct Reset;
  impl Drop for Reset {
    fn drop(&mut self) {
      LOCAL_DROP.with(|slot| { slot.take(); });
    }
  }
  let reset = Reset;
  let result = drag();
  drop(reset);
  (result, scope.position.get())
}

fn matching_local_drag(hwnd: HWND, data: &IDataObject) -> Option<Rc<LocalDropObservation>> {
  // No thread-local borrow is held across COM QueryInterface or other foreign code.
  let scope = LOCAL_DROP.with(|slot| slot.borrow().clone())?;
  if !unsafe { IsChild(scope.owner, hwnd) }.as_bool() { return None; }
  let source = data.cast::<IUnknown>().ok()?;
  (source.as_raw() == scope.source.as_raw() && scope.is_current(hwnd)).then_some(scope)
}

impl LocalDropObservation {
  fn is_current(&self, hwnd: HWND) -> bool {
    LOCAL_DROP.with(|slot| slot.borrow().as_deref().is_some_and(|scope| ptr::eq(scope, self)))
      && unsafe { IsChild(self.owner, hwnd) }.as_bool()
  }

  fn capture(&self, hwnd: HWND, pt: &POINTL) -> bool {
    let mut point = POINT { x: pt.x, y: pt.y };
    if self.position.get().is_some() || !unsafe { ScreenToClient(hwnd, &mut point) }.as_bool() {
      return false;
    }
    self.position.set(Some((point.x, point.y)));
    true
  }
}

#[derive(Default)]
pub(crate) struct DragDropController {
  drop_targets: Vec<IDropTarget>,
}

impl DragDropController {
  #[inline]
  pub(crate) fn new(hwnd: HWND, handler: Box<dyn Fn(DragDropEvent) -> bool>) -> Self {
    let mut controller = DragDropController::default();

    let handler = Rc::new(handler);

    // Enumerate child windows to find the WebView2 "window" and override!
    {
      let mut callback = |hwnd| controller.inject_in_hwnd(hwnd, handler.clone());
      let mut trait_obj: &mut dyn FnMut(HWND) -> bool = &mut callback;
      let closure_pointer_pointer: *mut c_void = unsafe { std::mem::transmute(&mut trait_obj) };
      let lparam = LPARAM(closure_pointer_pointer as _);
      unsafe extern "system" fn enumerate_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let closure = &mut *(lparam.0 as *mut c_void as *mut &mut dyn FnMut(HWND) -> bool);
        closure(hwnd).into()
      }
      let _ = unsafe { EnumChildWindows(Some(hwnd), Some(enumerate_callback), lparam) };
    }

    controller
  }

  #[inline]
  fn inject_in_hwnd(&mut self, hwnd: HWND, handler: Rc<dyn Fn(DragDropEvent) -> bool>) -> bool {
    let drag_drop_target: IDropTarget = DragDropTarget::new(hwnd, handler).into();
    if unsafe { RevokeDragDrop(hwnd) } != Err(DRAGDROP_E_INVALIDHWND.into())
      && unsafe { RegisterDragDrop(hwnd, &drag_drop_target) }.is_ok()
    {
      self.drop_targets.push(drag_drop_target);
    }

    true
  }
}

#[implement(IDropTarget)]
pub struct DragDropTarget {
  hwnd: HWND,
  listener: Rc<dyn Fn(DragDropEvent) -> bool>,
  hover: RefCell<DragHover>,
}

enum DragHover {
  None,
  External,
  Local(Weak<LocalDropObservation>),
}

// OLE supplies the source's allowed effects; return only the effect we select.
fn take_copy_permission(effect: &mut DROPEFFECT) -> bool {
  let allowed = effect.0 & DROPEFFECT_COPY.0 != 0;
  *effect = DROPEFFECT_NONE;
  allowed
}

impl DragDropTarget {
  pub fn new(hwnd: HWND, listener: Rc<dyn Fn(DragDropEvent) -> bool>) -> DragDropTarget {
    Self {
      hwnd,
      listener,
      hover: RefCell::new(DragHover::None),
    }
  }

  unsafe fn iterate_filenames<F>(
    data_obj: windows_core::Ref<'_, IDataObject>,
    mut callback: F,
  ) -> Option<HDROP>
  where
    F: FnMut(PathBuf),
  {
    let drop_format = FORMATETC {
      cfFormat: CF_HDROP.0,
      ptd: ptr::null_mut(),
      dwAspect: DVASPECT_CONTENT.0,
      lindex: -1,
      tymed: TYMED_HGLOBAL.0 as u32,
    };

    match data_obj.as_ref()?.GetData(&drop_format)
    {
      Ok(medium) => {
        let hdrop = HDROP(medium.u.hGlobal.0 as _);

        // The second parameter (0xFFFFFFFF) instructs the function to return the item count
        let item_count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);

        for i in 0..item_count {
          // Get the length of the path string NOT including the terminating null character.
          // Previously, this was using a fixed size array of MAX_PATH length, but the
          // Windows API allows longer paths under certain circumstances.
          let character_count = DragQueryFileW(hdrop, i, None) as usize;

          // Fill path_buf with the null-terminated file name
          let str_len = character_count + 1;
          let mut path_buf = vec![0; str_len];
          DragQueryFileW(hdrop, i, Some(&mut path_buf));
          callback(OsString::from_wide(&path_buf[0..character_count]).into());
        }

        Some(hdrop)
      }
      Err(_error) => {
        #[cfg(feature = "tracing")]
        tracing::warn!(
          "{}",
          match _error.code() {
            windows::Win32::Foundation::DV_E_FORMATETC => {
              // If the dropped item is not a file this error will occur.
              // In this case it is OK to return without taking further action.
              "Error occurred while processing dropped/hovered item: item is not a file."
            }
            _ => "Unexpected error occurred while processing dropped/hovered item.",
          }
        );
        None
      }
    }
  }
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for DragDropTarget_Impl {
  fn DragEnter(
    &self,
    pDataObj: windows_core::Ref<'_, IDataObject>,
    _grfKeyState: MODIFIERKEYS_FLAGS,
    pt: &POINTL,
    pdwEffect: *mut DROPEFFECT,
  ) -> windows::core::Result<()> {
    self.hover.replace(DragHover::None);
    let Some(effect) = (unsafe { pdwEffect.as_mut() }) else { return Ok(()) };
    let copy_allowed = take_copy_permission(effect);
    if let Some(scope) = pDataObj.as_ref().and_then(|data| matching_local_drag(self.hwnd, data)) {
      if copy_allowed && scope.position.get().is_none() {
        *effect = DROPEFFECT_COPY;
      }
      self.hover.replace(DragHover::Local(Rc::downgrade(&scope)));
      return Ok(());
    }
    if !copy_allowed { return Ok(()); }
    let mut pt = POINT { x: pt.x, y: pt.y };
    let _ = unsafe { ScreenToClient(self.hwnd, &mut pt) };

    let mut paths = Vec::new();
    let hdrop = unsafe { DragDropTarget::iterate_filenames(pDataObj, |path| paths.push(path)) };

    if hdrop.is_none() { return Ok(()); }
    self.hover.replace(DragHover::External);
    *effect = DROPEFFECT_COPY;

    (self.listener)(DragDropEvent::Enter {
      paths,
      position: (pt.x as _, pt.y as _),
    });

    Ok(())
  }

  fn DragOver(
    &self,
    _grfKeyState: MODIFIERKEYS_FLAGS,
    pt: &POINTL,
    pdwEffect: *mut DROPEFFECT,
  ) -> windows::core::Result<()> {
    let Some(effect) = (unsafe { pdwEffect.as_mut() }) else { return Ok(()) };
    if !take_copy_permission(effect) { return Ok(()); }
    let external = match &*self.hover.borrow() {
      DragHover::Local(scope) => {
        if scope.upgrade().is_some_and(|scope| scope.is_current(self.hwnd) && scope.position.get().is_none()) {
          *effect = DROPEFFECT_COPY;
        }
        false
      }
      DragHover::External => true,
      DragHover::None => false,
    };
    if external {
      *effect = DROPEFFECT_COPY;
      let mut pt = POINT { x: pt.x, y: pt.y };
      let _ = unsafe { ScreenToClient(self.hwnd, &mut pt) };
      (self.listener)(DragDropEvent::Over {
        position: (pt.x as _, pt.y as _),
      });
    }

    Ok(())
  }

  fn DragLeave(&self) -> windows::core::Result<()> {
    if matches!(self.hover.replace(DragHover::None), DragHover::External) {
      (self.listener)(DragDropEvent::Leave);
    }
    Ok(())
  }

  fn Drop(
    &self,
    pDataObj: windows_core::Ref<'_, IDataObject>,
    _grfKeyState: MODIFIERKEYS_FLAGS,
    pt: &POINTL,
    pdwEffect: *mut DROPEFFECT,
  ) -> windows::core::Result<()> {
    let hover = self.hover.replace(DragHover::None);
    let Some(effect) = (unsafe { pdwEffect.as_mut() }) else { return Ok(()) };
    if !take_copy_permission(effect) { return Ok(()); }
    if let Some(scope) = pDataObj.as_ref().and_then(|data| matching_local_drag(self.hwnd, data)) {
      // Recheck COM identity; a local transfer never enters the generic path.
      if matches!(hover, DragHover::Local(entered) if entered.ptr_eq(&Rc::downgrade(&scope)))
        && scope.capture(self.hwnd, pt)
      {
        *effect = DROPEFFECT_COPY;
      }
      return Ok(());
    }
    if matches!(hover, DragHover::External) {
      let mut pt = POINT { x: pt.x, y: pt.y };
      let _ = unsafe { ScreenToClient(self.hwnd, &mut pt) };

      let mut paths = Vec::new();
      let hdrop = unsafe { DragDropTarget::iterate_filenames(pDataObj, |path| paths.push(path)) };
      if let Some(hdrop) = hdrop {
        unsafe { DragFinish(hdrop) };
        *effect = DROPEFFECT_COPY;
        (self.listener)(DragDropEvent::Drop {
          paths,
          position: (pt.x as _, pt.y as _),
        });
      }
    }

    Ok(())
  }
}
