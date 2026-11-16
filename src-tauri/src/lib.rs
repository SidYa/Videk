//! Videk: a minimal video player. libmpv decodes and renders into a native child window
//! (host.rs) under a transparent WebView2 that draws the whole UI (title bar with the
//! controls, menus, settings). The UI talks to mpv through the commands below and gets the
//! observed mpv properties as `mpv-prop` events. Settings (settings.rs), opening files
//! (media.rs) and the accent colours (accent.rs) are handled here too: the UI only shows.

mod accent;
#[cfg(windows)]
mod assoc;
#[cfg(windows)]
mod drop;
mod host;
mod media;
mod mpv;
mod settings;

use mpv::Mpv;
use serde::Serialize;
use serde_json::Value;
use settings::Store;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, DragDropEvent, Emitter, Manager, WebviewWindow, WindowEvent};

/// Height of the title bar (CSS --header-h): in a window the video starts below it.
const HEADER_H: f64 = 36.0;

/// Properties mirrored to the UI.
const OBSERVED: &[&str] = &[
    "pause",
    "time-pos",
    "duration",
    "volume",
    "mute",
    "speed",
    "media-title",
    "path",
    "idle-active",
    "eof-reached",
    "seeking",
    "paused-for-cache",
    "demuxer-cache-time",
    "loop-file",
    "track-list",
    "chapter-list",
    "playlist-count",
    "playlist-pos",
    "dwidth",
    "dheight",
    "aid",
    "sid",
    "hwdec-current",
];

/// time-pos changes on every frame: send it at most this often (the last one is never lost).
const TIME_POS_INTERVAL: Duration = Duration::from_millis(50);

type Cache = Arc<Mutex<HashMap<String, Value>>>;

pub(crate) struct Player {
    pub mpv: Arc<Mpv>,
    pub settings: Arc<Store>,
    /// The main window and the video window in it (HWNDs).
    parent: isize,
    host: isize,
    /// Latest value of each observed property (the UI reads it once at start).
    cache: Cache,
    quitting: AtomicBool,
}

impl Player {
    /// No file is open.
    pub fn is_idle(&self) -> bool {
        self.cache.lock().unwrap().get("idle-active").and_then(Value::as_bool).unwrap_or(true)
    }

    fn save_position(&self) {
        if !self.is_idle() && self.settings.get().resume {
            let _ = self.mpv.command(&["write-watch-later-config"]);
        }
    }
}

#[derive(Clone, Serialize)]
struct PropEvent<'a> {
    name: &'a str,
    value: &'a Value,
}

/// mpv's MPV_END_FILE_REASON_ERROR.
const END_FILE_ERROR: i32 = 4;

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

/// Paths among command-line arguments (made absolute against `cwd`).
fn file_args(args: impl IntoIterator<Item = String>, cwd: &Path) -> Vec<String> {
    args.into_iter()
        .filter(|a| !a.starts_with('-'))
        .filter_map(|a| {
            if a.contains("://") {
                return Some(a);
            }
            let p = PathBuf::from(&a);
            let p = if p.is_absolute() { p } else { cwd.join(p) };
            // Only existing files (`tauri dev` passes cargo's own flags through, e.g. "always").
            p.exists().then(|| p.to_string_lossy().into_owned())
        })
        .collect()
}

/// Puts the video surface under the title bar (window) or over the whole client area
/// (full screen: the title bar then slides over the video).
fn layout(window: &WebviewWindow, host: isize) {
    let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) else { return };
    SCALE.store(scale.to_bits(), Ordering::Relaxed);
    let full = window.is_fullscreen().unwrap_or(false);
    let top = if full { 0 } else { (HEADER_H * scale).round() as i32 };
    let (w, h) = (size.width as i32, size.height as i32);
    #[cfg(windows)]
    {
        host::place(host, 0, top, w, h - top);
        host::set_drag_enabled(!full);
        if let Ok(parent) = window.hwnd() {
            host::set_resize_borders(parent.0 as isize, !full);
        }
    }
}

/// The window's scale factor (physical / CSS pixels), kept by `layout`.
static SCALE: AtomicU64 = AtomicU64::new(0x3FF0_0000_0000_0000); // 1.0

fn scale() -> f64 {
    f64::from_bits(SCALE.load(Ordering::Relaxed))
}

#[derive(Clone, Serialize)]
struct VideoMouse {
    kind: &'static str,
    x: f64,
    y: f64,
    delta: i32,
}

/// Input on the video (outside the WebView region) → `video-mouse` / `open-files` events.
fn video_input(app: AppHandle) -> Box<dyn Fn(host::Input) + Send + Sync> {
    Box::new(move |input| match input {
        host::Input::Mouse { kind, x, y, delta } => {
            let s = scale();
            let _ = app.emit("video-mouse", VideoMouse { kind, x: x as f64 / s, y: y as f64 / s, delta });
        }
        host::Input::Drop(files) => media::open_paths(&app, files),
    })
}

fn start_mpv(app: &AppHandle, host: isize, prefs: &settings::Settings) -> Result<Mpv, String> {
    let mpv = Mpv::load(&exe_dir().join("libmpv-2.dll"))?;
    let path = app.path();
    let watch_later = path.app_data_dir().map_err(|e| e.to_string())?.join("watch_later");
    let _ = std::fs::create_dir_all(&watch_later);
    let shots = path.picture_dir().map(|d| d.join("Videk")).unwrap_or_else(|_| exe_dir());

    let opts: &[(&str, String)] = &[
        ("wid", host.to_string()),
        ("vo", "gpu-next".into()),
        ("volume", prefs.volume.round().to_string()),
        ("keep-open", "yes".into()),
        ("idle", "yes".into()),
        ("force-window", "yes".into()),
        ("osc", "no".into()),
        ("osd-level", "0".into()),
        ("input-default-bindings", "no".into()),
        ("input-vo-keyboard", "no".into()),
        ("input-cursor", "no".into()),
        // mpv's own drop target (on its window) would swallow the drops: drop.rs handles them.
        ("drag-and-drop", "no".into()),
        ("cursor-autohide", "no".into()),
        ("ytdl", "no".into()),
        ("audio-display", "embedded-first".into()),
        ("sub-auto", "fuzzy".into()),
        ("watch-later-directory", watch_later.to_string_lossy().into_owned()),
        ("screenshot-directory", shots.to_string_lossy().into_owned()),
        ("screenshot-template", "%F-%wH.%wM.%wS".into()),
        ("screenshot-format", "png".into()),
        ("directory-filter-types", "video,audio".into()),
        ("volume-max", "150".into()),
    ];
    for (k, v) in opts.iter().chain(prefs.mpv_values().iter()) {
        mpv.set_option(k, v)?;
    }
    mpv.initialize()?;
    if cfg!(debug_assertions) {
        mpv.request_log_messages("warn");
    }
    for p in OBSERVED {
        mpv.observe(p)?;
    }
    Ok(mpv)
}

/// The mpv event loop (own thread): forwards property changes to the UI, sizes the window to
/// each new video, reports files that do not open. On shutdown (the `quit` sent when the
/// window closes) remembers the volume, destroys mpv, then ends the app.
fn event_loop(app: AppHandle, mpv: Arc<Mpv>, cache: Cache, settings: Arc<Store>, host: isize) {
    #[cfg(windows)]
    host::disable_children(host);
    // A file started: the window is fitted to its video once the video size is known.
    let mut need_fit = false;
    let mut pending_time: Option<Value> = None;
    let mut last_time = Instant::now() - TIME_POS_INTERVAL;
    loop {
        let timeout = if pending_time.is_some() {
            TIME_POS_INTERVAL.saturating_sub(last_time.elapsed()).as_secs_f64()
        } else {
            -1.0
        };
        let ev = mpv.wait_event(timeout);
        match ev.event_id {
            mpv::EVENT_SHUTDOWN => break,
            mpv::EVENT_PROPERTY_CHANGE => {
                let p = unsafe { &*(ev.data as *const mpv::EventProperty) };
                let name = mpv::cstr_lossy(p.name);
                let value = unsafe { mpv::property_value(p) };
                cache.lock().unwrap().insert(name.clone(), value.clone());
                if name == "time-pos" && last_time.elapsed() < TIME_POS_INTERVAL {
                    pending_time = Some(value);
                } else {
                    if name == "time-pos" {
                        last_time = Instant::now();
                        pending_time = None;
                    }
                    let _ = app.emit("mpv-prop", PropEvent { name: &name, value: &value });
                }
            }
            mpv::EVENT_START_FILE => need_fit = true,
            mpv::EVENT_FILE_LOADED => {
                if let Ok(Value::String(title)) = mpv.get_property("media-title") {
                    media::osd(&app, title);
                }
            }
            mpv::EVENT_VIDEO_RECONFIG => {
                // mpv may (re)create its window: keep it out of the mouse's way.
                #[cfg(windows)]
                host::disable_children(host);
                if need_fit {
                    let dim = |name| mpv.get_property(name).ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let (w, h) = (dim("dwidth"), dim("dheight"));
                    if w > 0.0 && h > 0.0 {
                        need_fit = false;
                        if settings.get().autofit {
                            if let Some(window) = app.get_webview_window("main") {
                                media::fit_window(&window, w, h, HEADER_H);
                            }
                        }
                    }
                }
            }
            mpv::EVENT_END_FILE => {
                let e = unsafe { &*(ev.data as *const mpv::EventEndFile) };
                if e.reason == END_FILE_ERROR {
                    media::osd(&app, "Не вдалося відкрити файл");
                }
            }
            mpv::EVENT_LOG_MESSAGE => {
                let m = unsafe { &*(ev.data as *const mpv::EventLogMessage) };
                eprint!("[mpv/{}] {}: {}", mpv::cstr_lossy(m.prefix), mpv::cstr_lossy(m.level), mpv::cstr_lossy(m.text));
            }
            _ => {}
        }
        if pending_time.is_some() && last_time.elapsed() >= TIME_POS_INTERVAL {
            let v = pending_time.take().unwrap();
            last_time = Instant::now();
            let _ = app.emit("mpv-prop", PropEvent { name: "time-pos", value: &v });
        }
    }
    if let Some(volume) = cache.lock().unwrap().get("volume").and_then(Value::as_f64) {
        settings.set_volume(volume);
    }
    mpv.destroy();
    app.exit(0);
}

// ---- commands -------------------------------------------------------------------------

#[tauri::command]
fn mpv_command(state: tauri::State<Player>, args: Vec<String>) -> Result<(), String> {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    state.mpv.command(&refs)
}

#[tauri::command]
fn mpv_set(state: tauri::State<Player>, name: String, value: Value) -> Result<(), String> {
    let v = match value {
        Value::Bool(b) => (if b { "yes" } else { "no" }).to_string(),
        Value::String(s) => s,
        Value::Null => "no".to_string(),
        other => other.to_string(),
    };
    state.mpv.set_property(&name, &v)
}

#[tauri::command]
fn mpv_get(state: tauri::State<Player>, name: String) -> Result<Value, String> {
    state.mpv.get_property(&name)
}

#[tauri::command]
fn mpv_snapshot(state: tauri::State<Player>) -> HashMap<String, Value> {
    state.cache.lock().unwrap().clone()
}

#[tauri::command]
fn settings_get(state: tauri::State<Player>) -> settings::Settings {
    state.settings.get()
}

/// Changes some settings (camelCase fields); the mpv ones apply at once.
#[tauri::command]
fn settings_set(state: tauri::State<Player>, patch: Value) -> Result<settings::Settings, String> {
    state.settings.patch(patch, Some(&state.mpv))
}

/// The accent colours for the current settings on a theme.
#[tauri::command]
fn look(state: tauri::State<Player>, dark: bool) -> accent::Look {
    let s = state.settings.get();
    accent::look(&s.accent, &s.custom_accent, dark)
}

#[tauri::command]
fn media_formats() -> media::Formats {
    media::formats()
}

#[tauri::command]
fn open_dialog(app: AppHandle) {
    media::open_dialog(&app);
}

#[tauri::command]
fn subtitles_dialog(app: AppHandle) {
    media::subtitles_dialog(&app);
}

#[tauri::command]
fn open_paths(app: AppHandle, paths: Vec<String>) {
    media::open_paths(&app, paths);
}

/// Stops playback (remembering the position when asked).
#[tauri::command]
fn player_stop(state: tauri::State<Player>) -> Result<(), String> {
    state.save_position();
    state.mpv.command(&["stop"])
}

/// The next / previous file of the playlist (remembering the position when asked).
#[tauri::command]
fn playlist_step(state: tauri::State<Player>, forward: bool) -> Result<(), String> {
    state.save_position();
    state.mpv.command(&[if forward { "playlist-next" } else { "playlist-prev" }])
}

#[derive(serde::Deserialize)]
struct UiRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    #[serde(default)]
    r: f64,
}

/// The parts of the window the UI draws (CSS pixels): the WebView is clipped to them, the
/// video shows everywhere else.
#[tauri::command]
fn set_ui_region(state: tauri::State<Player>, rects: Vec<UiRect>) {
    let s = scale();
    let px: Vec<host::Rect> = rects
        .iter()
        .map(|r| host::Rect {
            x: (r.x * s).floor() as i32,
            y: (r.y * s).floor() as i32,
            w: (r.w * s).ceil() as i32,
            h: (r.h * s).ceil() as i32,
            r: (r.r * s).round() as i32,
        })
        .collect();
    #[cfg(windows)]
    host::set_region(state.parent, state.host, &px);
}

#[cfg(windows)]
#[tauri::command]
fn assoc_status() -> assoc::Status {
    assoc::status()
}

#[cfg(windows)]
#[tauri::command]
fn assoc_apply(selected: Vec<String>) -> Result<assoc::Status, String> {
    assoc::apply(&selected)
}

#[cfg(windows)]
#[tauri::command]
fn open_default_apps() -> Result<(), String> {
    assoc::open_default_apps()
}

#[tauri::command]
fn set_cursor_hidden(hidden: bool) {
    #[cfg(windows)]
    host::set_cursor_hidden(hidden);
}

#[tauri::command]
fn set_fullscreen(window: WebviewWindow, state: tauri::State<Player>, on: bool) -> Result<(), String> {
    window.set_fullscreen(on).map_err(|e| e.to_string())?;
    layout(&window, state.host);
    Ok(())
}

// ---- app ------------------------------------------------------------------------------

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let files = file_args(argv.into_iter().skip(1), Path::new(&cwd));
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            media::open_paths(app, files);
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");
            let parent = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
            let host = host::create(parent, video_input(app.handle().clone()))?;
            #[cfg(windows)]
            {
                let handle = app.handle().clone();
                if let Err(e) = drop::register(host, Box::new(move |files| media::open_paths(&handle, files))) {
                    eprintln!("{e}");
                }
            }
            layout(&window, host);

            let config = app.path().app_config_dir().map_err(|e| e.to_string())?;
            let settings = Arc::new(Store::load(config.join("settings.json")));
            let mpv = match start_mpv(app.handle(), host, &settings.get()) {
                Ok(m) => Arc::new(m),
                Err(e) => {
                    #[cfg(windows)]
                    host::message_box(&format!(
                        "Не вдалося запустити libmpv.\n\n{e}\n\nПереконайтеся, що libmpv-2.dll лежить поруч із videk.exe."
                    ));
                    std::process::exit(1);
                }
            };
            let cache = Arc::new(Mutex::new(HashMap::new()));
            {
                let (app, mpv, cache, settings) =
                    (app.handle().clone(), mpv.clone(), cache.clone(), settings.clone());
                std::thread::Builder::new()
                    .name("mpv-events".into())
                    .spawn(move || event_loop(app, mpv, cache, settings, host))?;
            }
            app.manage(Player { mpv, settings, parent, host, cache, quitting: AtomicBool::new(false) });
            // Files from the command line (Explorer: "Open with Videk"). mpv already has the
            // settings, so the first file opens with them.
            let cwd = std::env::current_dir().unwrap_or_default();
            media::open_paths(app.handle(), file_args(std::env::args().skip(1), &cwd));
            Ok(())
        })
        .on_window_event(|window, event| {
            let Some(player) = window.try_state::<Player>() else { return };
            match event {
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    if let Some(w) = window.app_handle().get_webview_window(window.label()) {
                        layout(&w, player.host);
                    }
                }
                // Files dropped on the UI parts (the video window has its own drop target).
                WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) => {
                    let paths = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                    media::open_paths(window.app_handle(), paths);
                }
                WindowEvent::CloseRequested { api, .. } => {
                    // mpv must be shut down (saving the position if asked) before the window
                    // goes: let the event loop do it, it then exits the app.
                    api.prevent_close();
                    if !player.quitting.swap(true, Ordering::SeqCst) {
                        let _ = window.hide();
                        if player.mpv.command(&["quit"]).is_err() {
                            window.app_handle().exit(0);
                        }
                        let app = window.app_handle().clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_secs(5));
                            app.exit(0);
                        });
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            mpv_command,
            mpv_set,
            mpv_get,
            mpv_snapshot,
            settings_get,
            settings_set,
            look,
            media_formats,
            open_dialog,
            subtitles_dialog,
            open_paths,
            player_stop,
            playlist_step,
            set_fullscreen,
            set_ui_region,
            set_cursor_hidden,
            assoc_status,
            assoc_apply,
            open_default_apps
        ])
        .run(tauri::generate_context!())
        .expect("error while running Videk");
}
