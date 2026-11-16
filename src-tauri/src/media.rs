//! Opening media: the formats Videk knows, what to do with a list of paths (files dropped,
//! from the command line, from the open dialogs), and sizing the window to a new video.

use crate::Player;
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

pub const VIDEO: &[&str] = &[
    "mkv", "mk3d", "mp4", "m4v", "mov", "avi", "webm", "flv", "f4v", "wmv", "asf", "mpg", "mpeg",
    "m2v", "ts", "m2ts", "mts", "vob", "3gp", "3g2", "ogv", "bik", "bk2", "rm", "rmvb", "divx",
];
pub const AUDIO: &[&str] = &[
    "mp3", "flac", "wav", "m4a", "aac", "ogg", "oga", "opus", "wma", "ape", "wv", "mka", "ac3",
    "dts", "alac", "aiff", "aif",
];
pub const SUBTITLES: &[&str] = &["srt", "ass", "ssa", "vtt", "sub", "idx", "sup", "smi"];

#[derive(Serialize)]
pub struct Formats {
    pub video: &'static [&'static str],
    pub audio: &'static [&'static str],
}

pub fn formats() -> Formats {
    Formats { video: VIDEO, audio: AUDIO }
}

fn ext(path: &str) -> String {
    Path::new(path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn is_subtitle(path: &str) -> bool {
    SUBTITLES.contains(&ext(path).as_str())
}

/// A short message over the video (lib/osd.svelte.ts).
pub fn osd(app: &AppHandle, text: impl Into<String>) {
    let _ = app.emit("osd", text.into());
}

/// Opens `paths`: subtitle files are added to the file that is playing; anything else
/// replaces what plays (the first) and is queued (the rest).
pub fn open_paths(app: &AppHandle, paths: Vec<String>) {
    let Some(player) = app.try_state::<Player>() else { return };
    if paths.is_empty() {
        return;
    }
    let playing = !player.is_idle();
    if playing && paths.iter().all(|p| is_subtitle(p)) {
        for p in &paths {
            let _ = player.mpv.command(&["sub-add", p, "select"]);
        }
        osd(app, if paths.len() == 1 { "Субтитри додано" } else { "Субтитри додано (кілька файлів)" });
        return;
    }
    if playing && player.settings.get().resume {
        let _ = player.mpv.command(&["write-watch-later-config"]);
    }
    for (i, p) in paths.iter().enumerate() {
        let _ = player.mpv.command(&["loadfile", p, if i == 0 { "replace" } else { "append" }]);
    }
    let _ = player.mpv.set_property("pause", "no");
}

/// The "Відкрити" dialog (several files), then `open_paths`.
pub fn open_dialog(app: &AppHandle) {
    let all: Vec<&str> = VIDEO.iter().chain(AUDIO).copied().collect();
    let handle = app.clone();
    app.dialog()
        .file()
        .set_title("Відкрити")
        .add_filter("Медіафайли", &all)
        .add_filter("Відео", VIDEO)
        .add_filter("Аудіо", AUDIO)
        .add_filter("Усі файли", &["*"])
        .pick_files(move |files| {
            let paths = files
                .unwrap_or_default()
                .into_iter()
                .filter_map(|f| f.into_path().ok())
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            open_paths(&handle, paths);
        });
}

/// The "Субтитри" dialog: adds the chosen files to the file that is playing.
pub fn subtitles_dialog(app: &AppHandle) {
    let handle = app.clone();
    app.dialog()
        .file()
        .set_title("Субтитри")
        .add_filter("Субтитри", SUBTITLES)
        .add_filter("Усі файли", &["*"])
        .pick_files(move |files| {
            let Some(player) = handle.try_state::<Player>() else { return };
            let mut n = 0;
            for f in files.unwrap_or_default() {
                if let Ok(p) = f.into_path() {
                    let _ = player.mpv.command(&["sub-add", &p.to_string_lossy(), "select"]);
                    n += 1;
                }
            }
            if n > 0 {
                osd(&handle, "Субтитри додано");
            }
        });
}

/// Sizes the window so the video shows at 1:1 (at most 80% of the screen's work area),
/// keeping the window's centre where it was. Not in full screen or maximized.
pub fn fit_window(window: &WebviewWindow, vw: f64, vh: f64, header_css: f64) {
    if vw <= 0.0 || vh <= 0.0 {
        return;
    }
    if window.is_fullscreen().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
        return;
    }
    let Ok(Some(mon)) = window.current_monitor() else { return };
    let area = mon.work_area();
    let (ax, ay) = (area.position.x as f64, area.position.y as f64);
    let (aw, ah) = (area.size.width as f64, area.size.height as f64);
    let scale = mon.scale_factor();
    let header = (header_css * scale).round();

    let k = (aw * 0.8 / vw).min((ah * 0.8 - header) / vh).min(1.0);
    let w = (vw * k).round().max((420.0 * scale).round()).min(aw);
    let h = ((w / vw) * vh + header).round().max((240.0 * scale).round()).min(ah);

    let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else { return };
    let cx = pos.x as f64 + size.width as f64 / 2.0;
    let cy = pos.y as f64 + size.height as f64 / 2.0;
    let x = (cx - w / 2.0).clamp(ax, (ax + aw - w).max(ax)).round();
    let y = (cy - h / 2.0).clamp(ay, (ay + ah - h).max(ay)).round();
    let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
    let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
}
