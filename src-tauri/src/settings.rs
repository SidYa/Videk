//! Settings (context menu > Налаштування), kept in `settings.json` in the app's config
//! folder. Rust owns them: the ones that live in mpv are applied before mpv starts (so the
//! first file already opens with them) and again whenever the UI changes one.

use crate::mpv::Mpv;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// "system" | "light" | "dark".
    pub theme: String,
    /// A name of `accent::ACCENTS` or "custom".
    pub accent: String,
    /// `#rrggbb` of the "custom" accent.
    pub custom_accent: String,
    /// Hardware decoding (mpv hwdec=auto-safe).
    pub hwdec: bool,
    /// Remember the position of each file and resume from it.
    pub resume: bool,
    /// Play the other files of the folder as a playlist.
    pub folder_playlist: bool,
    /// Size the window to the video when a file opens.
    pub autofit: bool,
    /// A click on the video pauses / plays.
    pub click_pause: bool,
    /// Volume of the last session (0..150).
    pub volume: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "system".into(),
            accent: "blue".into(),
            custom_accent: "#e05a9c".into(),
            hwdec: true,
            resume: true,
            folder_playlist: true,
            autofit: true,
            click_pause: false,
            volume: 100.0,
        }
    }
}

impl Settings {
    /// mpv options / properties that follow the settings, as (name, value).
    pub fn mpv_values(&self) -> [(&'static str, String); 4] {
        let yes_no = |b: bool| if b { "yes" } else { "no" }.to_string();
        [
            ("hwdec", if self.hwdec { "auto-safe" } else { "no" }.into()),
            ("save-position-on-quit", yes_no(self.resume)),
            ("resume-playback", yes_no(self.resume)),
            ("autocreate-playlist", if self.folder_playlist { "filter" } else { "no" }.into()),
        ]
    }

    fn sanitize(&mut self) {
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            self.theme = "system".into();
        }
        if self.accent != "custom" && !crate::accent::ACCENTS.iter().any(|a| a.0 == self.accent) {
            self.accent = "blue".into();
        }
        if crate::accent::parse_hex(&self.custom_accent).is_none() {
            self.custom_accent = "#e05a9c".into();
        }
        self.volume = if self.volume.is_finite() { self.volume.clamp(0.0, 150.0) } else { 100.0 };
    }
}

pub struct Store {
    path: PathBuf,
    data: Mutex<Settings>,
}

impl Store {
    /// Reads `path` (missing or broken: the defaults).
    pub fn load(path: PathBuf) -> Self {
        let mut data: Settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        data.sanitize();
        Store { path, data: Mutex::new(data) }
    }

    pub fn get(&self) -> Settings {
        self.data.lock().unwrap().clone()
    }

    fn save(&self, s: &Settings) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(s) {
            let _ = std::fs::write(&self.path, json);
        }
    }

    /// Merges `patch` (some fields, camelCase) into the settings, saves them and applies
    /// the mpv ones that changed.
    pub fn patch(&self, patch: Value, mpv: Option<&Mpv>) -> Result<Settings, String> {
        let Value::Object(fields) = patch else { return Err("settings patch: not an object".into()) };
        let mut guard = self.data.lock().unwrap();
        let before = guard.clone();
        let mut merged = serde_json::to_value(&*guard).map_err(|e| e.to_string())?;
        if let Value::Object(m) = &mut merged {
            m.extend(fields);
        }
        let mut next: Settings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
        next.sanitize();
        if let Some(mpv) = mpv {
            for ((name, new), (_, old)) in next.mpv_values().iter().zip(before.mpv_values().iter()) {
                if new != old {
                    let _ = mpv.set_property(name, new);
                }
            }
        }
        *guard = next.clone();
        drop(guard);
        self.save(&next);
        Ok(next)
    }

    /// Remembers the volume (on quit).
    pub fn set_volume(&self, volume: f64) {
        let mut guard = self.data.lock().unwrap();
        if (guard.volume - volume).abs() < 0.5 {
            return;
        }
        guard.volume = volume;
        let s = guard.clone();
        drop(guard);
        self.save(&s);
    }
}
