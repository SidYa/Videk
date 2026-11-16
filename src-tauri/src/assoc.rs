//! File associations (Settings > Асоціації файлів), per user (HKCU, no admin rights).
//!
//! Windows 10/11 do not let a program make itself the default silently (the user's choice is
//! protected by a hash). What a program may do, and this does:
//! - register ProgIDs `Videk.Video` / `Videk.Audio` and list them in each extension's
//!   OpenWithProgids ("Open with"), and as the extension's default where the user never chose
//!   one;
//! - declare the capabilities (RegisteredApplications), so Videk is listed in Settings >
//!   Default apps, where `open_default_apps` takes the user to confirm it.

use serde::{Deserialize, Serialize};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

const APP: &str = "Videk";
const CAPS: &str = r"Software\Videk\Capabilities";
const CLASSES: &str = r"Software\Classes";

#[derive(Deserialize)]
pub struct Selection {
    pub video: Vec<String>,
    pub audio: Vec<String>,
    /// Every extension Videk knows: the ones not selected are unregistered.
    pub all: Vec<String>,
}

#[derive(Serialize)]
pub struct Status {
    /// Extensions registered to Videk (without the dot).
    pub registered: Vec<String>,
    /// The exe the registration points to (it may be another copy of Videk).
    pub exe: Option<String>,
}

fn hkcu() -> RegKey {
    RegKey::predef(HKEY_CURRENT_USER)
}

fn exe() -> Result<String, String> {
    std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string())
}

fn notify() {
    use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}

fn prog_id(kind: &str) -> String {
    format!("{APP}.{kind}")
}

fn write_prog_id(kind: &str, description: &str, exe: &str) -> std::io::Result<()> {
    let (key, _) = hkcu().create_subkey(format!(r"{CLASSES}\{}", prog_id(kind)))?;
    key.set_value("", &description)?;
    key.set_value("FriendlyTypeName", &description)?;
    let (icon, _) = key.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{exe}\",0"))?;
    let (cmd, _) = key.create_subkey(r"shell\open\command")?;
    cmd.set_value("", &format!("\"{exe}\" \"%1\""))?;
    Ok(())
}

pub fn status() -> Status {
    let mut registered = Vec::new();
    if let Ok(fa) = hkcu().open_subkey_with_flags(format!(r"{CAPS}\FileAssociations"), KEY_READ) {
        for (name, _) in fa.enum_values().flatten() {
            registered.push(name.trim_start_matches('.').to_lowercase());
        }
    }
    let exe = hkcu()
        .open_subkey(format!(r"{CLASSES}\{}\shell\open\command", prog_id("Video")))
        .and_then(|k| k.get_value::<String, _>(""))
        .ok()
        .and_then(|cmd| cmd.split('"').nth(1).map(str::to_string));
    Status { registered, exe }
}

pub fn apply(sel: &Selection) -> Result<Status, String> {
    let exe = exe()?;
    let run = || -> std::io::Result<()> {
        let root = hkcu();
        write_prog_id("Video", "Відео (Videk)", &exe)?;
        write_prog_id("Audio", "Аудіо (Videk)", &exe)?;

        // "Open with" entry of the exe itself.
        let (app, _) = root.create_subkey(format!(r"{CLASSES}\Applications\videk.exe"))?;
        app.set_value("FriendlyAppName", &APP)?;
        let (cmd, _) = app.create_subkey(r"shell\open\command")?;
        cmd.set_value("", &format!("\"{exe}\" \"%1\""))?;
        let _ = app.delete_subkey_all("SupportedTypes");
        let (types, _) = app.create_subkey("SupportedTypes")?;

        // Capabilities: Videk in Settings > Default apps.
        let (caps, _) = root.create_subkey(CAPS)?;
        caps.set_value("ApplicationName", &APP)?;
        caps.set_value("ApplicationDescription", &"Швидкий мінімалістичний відеоплеєр")?;
        caps.set_value("ApplicationIcon", &format!("\"{exe}\",0"))?;
        let _ = caps.delete_subkey_all("FileAssociations");
        let (fa, _) = caps.create_subkey("FileAssociations")?;
        let (reg_apps, _) = root.create_subkey(r"Software\RegisteredApplications")?;
        reg_apps.set_value(APP, &CAPS)?;

        let selected = |e: &String| sel.video.contains(e) || sel.audio.contains(e);
        for e in &sel.all {
            let ext = format!(".{e}");
            let (ext_key, _) = root.create_subkey(format!(r"{CLASSES}\{ext}"))?;
            let kind = if sel.audio.contains(e) { "Audio" } else { "Video" };
            let id = prog_id(kind);
            let current: String = ext_key.get_value("").unwrap_or_default();
            if selected(e) {
                let (owp, _) = ext_key.create_subkey("OpenWithProgids")?;
                owp.set_value(&id, &"")?;
                fa.set_value(&ext, &id)?;
                types.set_value(&ext, &"")?;
                // The default where the user never chose a program (Explorer prefers the
                // user's choice when there is one).
                if current.is_empty() || current.starts_with("Videk.") {
                    ext_key.set_value("", &id)?;
                }
            } else {
                if let Ok(owp) = ext_key.open_subkey_with_flags("OpenWithProgids", KEY_READ | KEY_WRITE) {
                    let _ = owp.delete_value(prog_id("Video"));
                    let _ = owp.delete_value(prog_id("Audio"));
                }
                if current.starts_with("Videk.") {
                    let _ = ext_key.delete_value("");
                }
            }
        }
        Ok(())
    };
    run().map_err(|e| format!("Реєстр: {e}"))?;
    notify();
    Ok(status())
}

/// Opens Settings > Default apps on Videk's page (Windows 11; Windows 10 shows the list).
pub fn open_default_apps() -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(format!("ms-settings:defaultapps?registeredAppUser={APP}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
