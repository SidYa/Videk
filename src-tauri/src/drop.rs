//! OLE drop target of the video window: files dragged from Explorer onto the video (the
//! WebView, clipped to the UI, only gets the drops on the UI parts).

use windows::Win32::Foundation::{HWND, POINTL};
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, IDropTarget, IDropTarget_Impl,
    RegisterDragDrop, ReleaseStgMedium,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows_core::{Ref, implement};

type OnDrop = Box<dyn Fn(Vec<String>) + Send + Sync>;

#[implement(IDropTarget)]
struct Target {
    on_drop: OnDrop,
}

fn hdrop_format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

fn has_files(obj: &IDataObject) -> bool {
    unsafe { obj.QueryGetData(&hdrop_format()).is_ok() }
}

fn files(obj: &IDataObject) -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        let Ok(mut medium) = obj.GetData(&hdrop_format()) else { return out };
        let drop = HDROP(medium.u.hGlobal.0);
        let n = DragQueryFileW(drop, u32::MAX, None);
        for i in 0..n {
            let len = DragQueryFileW(drop, i, None) as usize;
            let mut buf = vec![0u16; len + 1];
            DragQueryFileW(drop, i, Some(&mut buf));
            out.push(String::from_utf16_lossy(&buf[..len]));
        }
        ReleaseStgMedium(&mut medium);
    }
    out
}

impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(
        &self,
        obj: Ref<IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        let ok = obj.as_ref().is_some_and(has_files);
        unsafe { *effect = if ok { DROPEFFECT_COPY } else { DROPEFFECT_NONE } };
        Ok(())
    }

    fn DragOver(
        &self,
        _keys: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        unsafe { *effect = DROPEFFECT_COPY };
        Ok(())
    }

    fn DragLeave(&self) -> windows_core::Result<()> {
        Ok(())
    }

    fn Drop(
        &self,
        obj: Ref<IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        unsafe { *effect = DROPEFFECT_COPY };
        if let Some(obj) = obj.as_ref() {
            let list = files(obj);
            if !list.is_empty() {
                (self.on_drop)(list);
            }
        }
        Ok(())
    }
}

/// Registers the drop target on `hwnd` (main thread; OLE is initialized there by tao).
pub fn register(hwnd: isize, on_drop: OnDrop) -> Result<(), String> {
    let target: IDropTarget = Target { on_drop }.into();
    unsafe { RegisterDragDrop(HWND(hwnd as *mut _), &target) }.map_err(|e| format!("RegisterDragDrop: {e}"))
}
