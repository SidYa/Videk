//! The native video surface and how it shares the window with the WebView.
//!
//! mpv renders (`wid`) into a black child window of the main window, kept at the bottom of
//! the z-order. A windowed WebView2 does not let sibling windows show through its
//! transparent pixels, so the WebView is clipped with a window region to the parts of the
//! UI that are on screen (title bar, menus, dialogs, OSD: `set_region`). Everywhere else the
//! video window is hit: its mouse input (and dropped files) is forwarded to the UI as
//! events. mpv's own child window is disabled, so its input lands here too.

#[cfg(windows)]
mod imp {
    use std::ptr::{null, null_mut};
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        BLACK_BRUSH, CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, GetStockObject,
        HBRUSH, MapWindowPoints, RGN_OR, ScreenToClient, SetWindowRgn,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, ReleaseCapture, SetCapture};
    use windows_sys::Win32::UI::Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    pub enum Input {
        Mouse { kind: &'static str, x: i32, y: i32, delta: i32 },
        Drop(Vec<String>),
    }

    static SINK: OnceLock<Box<dyn Fn(Input) + Send + Sync>> = OnceLock::new();
    static CURSOR_HIDDEN: AtomicBool = AtomicBool::new(false);
    /// Dragging the video moves the window (not in full screen).
    static DRAG_ENABLED: AtomicBool = AtomicBool::new(true);
    /// Pixels the mouse must travel with the left button down before the window moves (a
    /// click or double click stays a click).
    const DRAG_THRESHOLD: i32 = 4;

    thread_local! {
        /// Where the left button went down (screen pixels), while a drag may start.
        static PRESS: std::cell::Cell<Option<(i32, i32)>> = const { std::cell::Cell::new(None) };
    }

    pub fn set_drag_enabled(on: bool) {
        DRAG_ENABLED.store(on, Ordering::Relaxed);
    }

    fn cursor_pos() -> (i32, i32) {
        let mut p = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut p) };
        (p.x, p.y)
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    fn lo(l: LPARAM) -> i32 {
        (l & 0xffff) as u16 as i16 as i32
    }
    fn hi(l: LPARAM) -> i32 {
        ((l >> 16) & 0xffff) as u16 as i16 as i32
    }

    fn send(i: Input) {
        if let Some(s) = SINK.get() {
            s(i);
        }
    }

    /// Host-client point → main-window client point (physical pixels).
    unsafe fn to_parent(hwnd: HWND, x: i32, y: i32) -> (i32, i32) {
        let mut p = POINT { x, y };
        unsafe { MapWindowPoints(hwnd, GetParent(hwnd), &mut p, 1) };
        (p.x, p.y)
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        unsafe {
            let mouse = |kind: &'static str| {
                let (x, y) = to_parent(hwnd, lo(lp), hi(lp));
                send(Input::Mouse { kind, x, y, delta: 0 });
            };
            match msg {
                WM_MOUSEMOVE => {
                    if let Some((px, py)) = PRESS.get() {
                        let (cx, cy) = cursor_pos();
                        if (wp & 0x0001) != 0 /* MK_LBUTTON */
                            && ((cx - px).abs() > DRAG_THRESHOLD || (cy - py).abs() > DRAG_THRESHOLD)
                        {
                            // Hand the press to the window's own move loop (as on a title bar).
                            PRESS.set(None);
                            ReleaseCapture();
                            let top = GetAncestor(hwnd, GA_ROOT);
                            let pos = ((cy as u32 & 0xffff) << 16 | (cx as u32 & 0xffff)) as LPARAM;
                            SendMessageW(top, WM_NCLBUTTONDOWN, HTCAPTION as WPARAM, pos);
                            return 0;
                        }
                    }
                    mouse("move");
                }
                WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                    if DRAG_ENABLED.load(Ordering::Relaxed) {
                        PRESS.set(Some(cursor_pos()));
                        SetCapture(hwnd);
                    }
                    mouse(if msg == WM_LBUTTONDOWN { "down" } else { "dblclick" });
                }
                WM_LBUTTONUP => {
                    if PRESS.take().is_some() {
                        ReleaseCapture();
                    }
                    mouse("up");
                }
                WM_CAPTURECHANGED => PRESS.set(None),
                WM_RBUTTONDOWN => mouse("rdown"),
                WM_RBUTTONUP => mouse("context"),
                WM_MBUTTONUP => mouse("middle"),
                WM_XBUTTONUP => mouse(if (wp >> 16) & 0xffff == 1 { "back" } else { "forward" }),
                WM_MOUSEWHEEL => {
                    let mut p = POINT { x: lo(lp), y: hi(lp) };
                    ScreenToClient(GetParent(hwnd), &mut p);
                    let delta = ((wp >> 16) & 0xffff) as u16 as i16 as i32;
                    send(Input::Mouse { kind: "wheel", x: p.x, y: p.y, delta });
                }
                WM_SETCURSOR => {
                    let cur = if CURSOR_HIDDEN.load(Ordering::Relaxed) {
                        null_mut()
                    } else {
                        LoadCursorW(null_mut(), IDC_ARROW)
                    };
                    SetCursor(cur);
                    return 1;
                }
                WM_DROPFILES => {
                    let drop = wp as HDROP;
                    let n = DragQueryFileW(drop, u32::MAX, null_mut(), 0);
                    let mut files = Vec::new();
                    for i in 0..n {
                        let len = DragQueryFileW(drop, i, null_mut(), 0);
                        let mut buf = vec![0u16; len as usize + 1];
                        DragQueryFileW(drop, i, buf.as_mut_ptr(), buf.len() as u32);
                        files.push(String::from_utf16_lossy(&buf[..len as usize]));
                    }
                    DragFinish(drop);
                    send(Input::Drop(files));
                    return 0;
                }
                _ => {}
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
    }

    pub fn create(parent: isize, sink: Box<dyn Fn(Input) + Send + Sync>) -> Result<isize, String> {
        let _ = SINK.set(sink);
        unsafe {
            let hinst = GetModuleHandleW(null());
            let class = wide("VidekVideoHost");
            let mut wc: WNDCLASSW = std::mem::zeroed();
            wc.style = CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS;
            wc.lpfnWndProc = Some(wndproc);
            wc.hInstance = hinst;
            wc.hbrBackground = GetStockObject(BLACK_BRUSH) as HBRUSH;
            wc.lpszClassName = class.as_ptr();
            RegisterClassW(&wc); // fails harmlessly if already registered
            let hwnd = CreateWindowExW(
                WS_EX_ACCEPTFILES,
                class.as_ptr(),
                null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
                0,
                0,
                1,
                1,
                parent as HWND,
                null_mut(),
                hinst,
                null(),
            );
            if hwnd.is_null() {
                return Err("CreateWindowExW (video host) failed".into());
            }
            DragAcceptFiles(hwnd, 1);
            Ok(hwnd as isize)
        }
    }

    /// Places the surface (client coordinates, physical pixels) under every sibling.
    pub fn place(host: isize, x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                host as HWND,
                HWND_BOTTOM,
                x,
                y,
                w.max(1),
                h.max(1),
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
        }
    }

    unsafe extern "system" fn disable_one(h: HWND, _: LPARAM) -> i32 {
        unsafe { EnableWindow(h, 0) };
        1
    }

    /// Disables mpv's window(s) inside the host: their mouse input goes to the host.
    pub fn disable_children(host: isize) {
        unsafe { EnumChildWindows(host as HWND, Some(disable_one), 0) };
    }

    pub fn set_cursor_hidden(hidden: bool) {
        CURSOR_HIDDEN.store(hidden, Ordering::Relaxed);
        if hidden {
            unsafe { SetCursor(null_mut()) };
        }
    }

    /// A rounded rectangle of the UI, physical pixels of the main window client area.
    pub struct Rect {
        pub x: i32,
        pub y: i32,
        pub w: i32,
        pub h: i32,
        pub r: i32,
    }

    fn class_name(h: HWND) -> String {
        let mut buf = [0u16; 64];
        let n = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    /// Clips the WebView (the WRY_WEBVIEW child of the main window) to `rects`. Other
    /// children (the video host, Tauri's TAURI_DRAG_RESIZE_BORDERS) are left alone.
    pub fn set_region(parent: isize, host: isize, rects: &[Rect]) {
        unsafe {
            let mut child = GetWindow(parent as HWND, GW_CHILD);
            while !child.is_null() {
                if child as isize != host && class_name(child) == "WRY_WEBVIEW" {
                    let rgn = CreateRectRgn(0, 0, 0, 0);
                    for r in rects {
                        let part = if r.r > 0 {
                            CreateRoundRectRgn(r.x, r.y, r.x + r.w + 1, r.y + r.h + 1, r.r * 2, r.r * 2)
                        } else {
                            CreateRectRgn(r.x, r.y, r.x + r.w, r.y + r.h)
                        };
                        CombineRgn(rgn, rgn, part, RGN_OR);
                        DeleteObject(part);
                    }
                    // The system owns the region from here.
                    SetWindowRgn(child, rgn, 1);
                }
                child = GetWindow(child, GW_HWNDNEXT);
            }
        }
    }

    pub fn message_box(text: &str) {
        let (t, c) = (wide(text), wide("Videk"));
        unsafe { MessageBoxW(null_mut(), t.as_ptr(), c.as_ptr(), MB_OK | MB_ICONERROR) };
    }
}

#[cfg(windows)]
pub use imp::*;
