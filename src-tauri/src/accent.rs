//! Accent colours (as in Farba): the palette per theme, and a colour adapted to the theme so
//! it always stands out from the backgrounds (WCAG 2 contrast for UI parts: 3:1).

use serde::Serialize;
use std::collections::BTreeMap;

/// (name, dark theme, light theme): about the Material / Tailwind 400 shade on the dark
/// theme and the 700 shade on the light one.
pub const ACCENTS: &[(&str, &str, &str)] = &[
    ("pink", "#e8607a", "#c2185b"),
    ("orange", "#f0913a", "#d0600a"),
    ("green", "#5cb860", "#2e7d32"),
    ("teal", "#2bb3a3", "#00897b"),
    ("blue", "#4b9be8", "#1f6fd1"),
    ("purple", "#a77be8", "#6a3fc8"),
    ("red", "#ef5350", "#c62828"),
    ("amber", "#f2b632", "#a86200"),
    ("lime", "#a5c94b", "#5a8a12"),
    ("cyan", "#37b9d8", "#00789e"),
    ("indigo", "#7f8cf0", "#3f51b5"),
];

/// Backgrounds the accent must stand out from, per theme (app.css: panels, popups, title
/// bar).
const BACKS_DARK: &[&str] = &["#2b2b2b", "#2f2f2f", "#1f1f1f"];
const BACKS_LIGHT: &[&str] = &["#f6f6f6", "#ffffff"];
const MIN_CONTRAST: f64 = 3.0;

type Rgb = [f64; 3];

pub fn parse_hex(hex: &str) -> Option<Rgb> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some([((n >> 16) & 255) as f64, ((n >> 8) & 255) as f64, (n & 255) as f64])
}

fn to_hex(c: Rgb) -> String {
    let b = |v: f64| v.round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}

/// Relative luminance (sRGB).
fn luminance(c: Rgb) -> f64 {
    let lin = |v: f64| {
        let s = v / 255.0;
        if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The accent as used on a theme: a colour too close to the theme's backgrounds is
/// lightened (dark theme) or darkened (light theme), keeping its hue, until it stands out.
pub fn adapt(hex: &str, dark: bool) -> String {
    let Some(c) = parse_hex(hex) else { return hex.to_string() };
    let backs: Vec<Rgb> =
        (if dark { BACKS_DARK } else { BACKS_LIGHT }).iter().filter_map(|h| parse_hex(h)).collect();
    let ok = |v: Rgb| backs.iter().all(|b| contrast(v, *b) >= MIN_CONTRAST);
    if ok(c) {
        return hex.to_string();
    }
    let to = if dark { 255.0 } else { 0.0 };
    for step in 1..=20 {
        let t = step as f64 * 0.05;
        let v = c.map(|x| x + (to - x) * t);
        if ok(v) {
            return to_hex(v);
        }
    }
    if dark { "#ffffff".into() } else { "#000000".into() }
}

/// Text on the accent (selected buttons...): white, or near black on a light accent.
fn text_on(c: Rgb) -> &'static str {
    if contrast(c, [255.0; 3]) >= contrast(c, [26.0; 3]) { "#ffffff" } else { "#1a1a1a" }
}

/// The CSS colours of the UI for the chosen accent on one theme, and the palette shown in
/// Settings.
#[derive(Serialize)]
pub struct Look {
    pub accent: String,
    pub accent_fg: String,
    pub accent_soft: String,
    /// Swatch colour of every accent (and "custom") on this theme.
    pub palette: BTreeMap<String, String>,
}

pub fn look(accent: &str, custom: &str, dark: bool) -> Look {
    let pick = |name: &str| -> String {
        ACCENTS
            .iter()
            .find(|a| a.0 == name)
            .map(|a| if dark { a.1 } else { a.2 }.to_string())
            .unwrap_or_else(|| custom.to_string())
    };
    let chosen = adapt(&pick(accent), dark);
    let c = parse_hex(&chosen).unwrap_or([75.0, 155.0, 232.0]);
    let mut palette: BTreeMap<String, String> = ACCENTS.iter().map(|a| (a.0.to_string(), pick(a.0))).collect();
    palette.insert("custom".into(), custom.to_string());
    Look {
        accent_fg: text_on(c).into(),
        accent_soft: format!(
            "rgba({}, {}, {}, {})",
            c[0] as u8,
            c[1] as u8,
            c[2] as u8,
            if dark { 0.28 } else { 0.2 }
        ),
        accent: chosen,
        palette,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_accents_are_kept() {
        for (_, d, l) in ACCENTS {
            assert_eq!(adapt(d, true), *d, "dark {d}");
            assert_eq!(adapt(l, false), *l, "light {l}");
        }
    }

    #[test]
    fn pale_colours_are_adapted() {
        let y = adapt("#ffff99", false);
        assert_ne!(y, "#ffff99");
        let c = parse_hex(&y).unwrap();
        assert!(contrast(c, [255.0; 3]) >= MIN_CONTRAST);
        assert_ne!(adapt("#101010", true), "#101010");
    }
}
