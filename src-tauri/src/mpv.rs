//! Minimal libmpv client API binding, loaded at run time from libmpv-2.dll (no import
//! library needed at build time). Only what Videk uses.

use libloading::Library;
use serde_json::{Map, Number, Value};
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::path::Path;

pub type Handle = *mut c_void;

pub const FORMAT_NONE: c_int = 0;
pub const FORMAT_STRING: c_int = 1;
pub const FORMAT_FLAG: c_int = 3;
pub const FORMAT_INT64: c_int = 4;
pub const FORMAT_DOUBLE: c_int = 5;
pub const FORMAT_NODE: c_int = 6;
pub const FORMAT_NODE_ARRAY: c_int = 7;
pub const FORMAT_NODE_MAP: c_int = 8;

pub const EVENT_SHUTDOWN: c_int = 1;
pub const EVENT_LOG_MESSAGE: c_int = 2;
pub const EVENT_START_FILE: c_int = 6;
pub const EVENT_END_FILE: c_int = 7;
pub const EVENT_FILE_LOADED: c_int = 8;
pub const EVENT_VIDEO_RECONFIG: c_int = 17;
pub const EVENT_PROPERTY_CHANGE: c_int = 22;

#[repr(C)]
pub struct Event {
    pub event_id: c_int,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct EventProperty {
    pub name: *const c_char,
    pub format: c_int,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct EventLogMessage {
    pub prefix: *const c_char,
    pub level: *const c_char,
    pub text: *const c_char,
    pub log_level: c_int,
}

#[repr(C)]
pub struct EventEndFile {
    pub reason: c_int,
    pub error: c_int,
    pub playlist_entry_id: i64,
    pub playlist_insert_id: i64,
    pub playlist_insert_num_entries: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union NodeU {
    pub string: *mut c_char,
    pub flag: c_int,
    pub int64: i64,
    pub double: c_double,
    pub list: *mut NodeList,
    pub ba: *mut c_void,
}

#[repr(C)]
pub struct Node {
    pub u: NodeU,
    pub format: c_int,
}

#[repr(C)]
pub struct NodeList {
    pub num: c_int,
    pub values: *mut Node,
    pub keys: *mut *mut c_char,
}

struct Api {
    _lib: Library,
    create: unsafe extern "C" fn() -> Handle,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    terminate_destroy: unsafe extern "C" fn(Handle),
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    command: unsafe extern "C" fn(Handle, *mut *const c_char) -> c_int,
    set_property_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    get_property: unsafe extern "C" fn(Handle, *const c_char, c_int, *mut c_void) -> c_int,
    free_node_contents: unsafe extern "C" fn(*mut Node),
    observe_property: unsafe extern "C" fn(Handle, u64, *const c_char, c_int) -> c_int,
    wait_event: unsafe extern "C" fn(Handle, c_double) -> *mut Event,
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
    request_log_messages: unsafe extern "C" fn(Handle, *const c_char) -> c_int,
}

/// An initialized mpv instance. The client API is thread-safe: commands and properties may
/// be used from any thread, while one thread runs `wait_event`.
pub struct Mpv {
    api: Api,
    h: Handle,
}

unsafe impl Send for Mpv {}
unsafe impl Sync for Mpv {}

fn cstr(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|_| format!("NUL in string: {s:?}"))
}

impl Mpv {
    /// Loads libmpv and creates an (uninitialized) instance.
    pub fn load(dll: &Path) -> Result<Self, String> {
        unsafe {
            let lib = Library::new(dll).map_err(|e| format!("{}: {e}", dll.display()))?;
            macro_rules! sym {
                ($name:literal) => {
                    *lib.get($name).map_err(|e| format!("{}: {e}", stringify!($name)))?
                };
            }
            let api = Api {
                create: sym!(b"mpv_create\0"),
                initialize: sym!(b"mpv_initialize\0"),
                terminate_destroy: sym!(b"mpv_terminate_destroy\0"),
                set_option_string: sym!(b"mpv_set_option_string\0"),
                command: sym!(b"mpv_command\0"),
                set_property_string: sym!(b"mpv_set_property_string\0"),
                get_property: sym!(b"mpv_get_property\0"),
                free_node_contents: sym!(b"mpv_free_node_contents\0"),
                observe_property: sym!(b"mpv_observe_property\0"),
                wait_event: sym!(b"mpv_wait_event\0"),
                error_string: sym!(b"mpv_error_string\0"),
                request_log_messages: sym!(b"mpv_request_log_messages\0"),
                _lib: lib,
            };
            let h = (api.create)();
            if h.is_null() {
                return Err("mpv_create failed".into());
            }
            Ok(Mpv { api, h })
        }
    }

    fn check(&self, rc: c_int, what: &str) -> Result<(), String> {
        if rc >= 0 {
            return Ok(());
        }
        let msg = unsafe { CStr::from_ptr((self.api.error_string)(rc)) };
        Err(format!("{what}: {}", msg.to_string_lossy()))
    }

    pub fn set_option(&self, name: &str, value: &str) -> Result<(), String> {
        let (n, v) = (cstr(name)?, cstr(value)?);
        let rc = unsafe { (self.api.set_option_string)(self.h, n.as_ptr(), v.as_ptr()) };
        self.check(rc, name)
    }

    pub fn initialize(&self) -> Result<(), String> {
        let rc = unsafe { (self.api.initialize)(self.h) };
        self.check(rc, "mpv_initialize")
    }

    pub fn command(&self, args: &[&str]) -> Result<(), String> {
        let owned = args.iter().map(|a| cstr(a)).collect::<Result<Vec<_>, _>>()?;
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        let rc = unsafe { (self.api.command)(self.h, ptrs.as_mut_ptr()) };
        self.check(rc, args.first().copied().unwrap_or("command"))
    }

    pub fn set_property(&self, name: &str, value: &str) -> Result<(), String> {
        let (n, v) = (cstr(name)?, cstr(value)?);
        let rc = unsafe { (self.api.set_property_string)(self.h, n.as_ptr(), v.as_ptr()) };
        self.check(rc, name)
    }

    pub fn get_property(&self, name: &str) -> Result<Value, String> {
        let n = cstr(name)?;
        let mut node = Node { u: NodeU { int64: 0 }, format: FORMAT_NONE };
        let rc = unsafe {
            (self.api.get_property)(self.h, n.as_ptr(), FORMAT_NODE, &mut node as *mut _ as *mut c_void)
        };
        self.check(rc, name)?;
        let v = unsafe { node_to_json(&node) };
        unsafe { (self.api.free_node_contents)(&mut node) };
        Ok(v)
    }

    pub fn observe(&self, name: &str) -> Result<(), String> {
        let n = cstr(name)?;
        let rc = unsafe { (self.api.observe_property)(self.h, 0, n.as_ptr(), FORMAT_NODE) };
        self.check(rc, name)
    }

    pub fn request_log_messages(&self, level: &str) {
        if let Ok(l) = cstr(level) {
            unsafe { (self.api.request_log_messages)(self.h, l.as_ptr()) };
        }
    }

    /// Waits for the next event (`timeout` in seconds, negative: forever). The returned
    /// event is valid until the next call. Only one thread may call this.
    pub fn wait_event(&self, timeout: f64) -> &Event {
        unsafe { &*(self.api.wait_event)(self.h, timeout) }
    }

    /// Destroys the instance. Must be the last call; no other thread may use it any more.
    pub fn destroy(&self) {
        unsafe { (self.api.terminate_destroy)(self.h) };
    }
}

/// Converts an mpv node (property value) to JSON.
///
/// # Safety
/// `node` must be a valid node from libmpv.
pub unsafe fn node_to_json(node: &Node) -> Value {
    unsafe {
        match node.format {
            FORMAT_STRING => Value::String(cstr_lossy(node.u.string)),
            FORMAT_FLAG => Value::Bool(node.u.flag != 0),
            FORMAT_INT64 => Value::Number(node.u.int64.into()),
            FORMAT_DOUBLE => Number::from_f64(node.u.double).map(Value::Number).unwrap_or(Value::Null),
            FORMAT_NODE_ARRAY => {
                let Some(l) = node.u.list.as_ref() else { return Value::Array(vec![]) };
                if l.num <= 0 || l.values.is_null() {
                    return Value::Array(vec![]);
                }
                let items = std::slice::from_raw_parts(l.values, l.num as usize);
                Value::Array(items.iter().map(|n| node_to_json(n)).collect())
            }
            FORMAT_NODE_MAP => {
                let Some(l) = node.u.list.as_ref() else { return Value::Object(Map::new()) };
                if l.num <= 0 || l.values.is_null() || l.keys.is_null() {
                    return Value::Object(Map::new());
                }
                let n = l.num as usize;
                let items = std::slice::from_raw_parts(l.values, n);
                let keys = std::slice::from_raw_parts(l.keys, n);
                let mut m = Map::new();
                for (k, v) in keys.iter().zip(items) {
                    m.insert(cstr_lossy(*k), node_to_json(v));
                }
                Value::Object(m)
            }
            _ => Value::Null,
        }
    }
}

/// The value of a property-change event (observed with FORMAT_NODE).
///
/// # Safety
/// `p` must come from a live PROPERTY_CHANGE event.
pub unsafe fn property_value(p: &EventProperty) -> Value {
    unsafe {
        if p.format == FORMAT_NODE && !p.data.is_null() {
            node_to_json(&*(p.data as *const Node))
        } else {
            Value::Null
        }
    }
}

pub fn cstr_lossy(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}
