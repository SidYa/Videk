// Settings, as shown by the UI. Rust keeps and applies them (src-tauri/src/settings.rs) and
// computes the accent colours (accent.rs); this mirrors them and writes the CSS variables.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type ThemeMode = "system" | "light" | "dark";
export type Accent =
  | "blue"
  | "teal"
  | "green"
  | "orange"
  | "pink"
  | "purple"
  | "red"
  | "amber"
  | "lime"
  | "cyan"
  | "indigo"
  | "custom";

/** The Settings swatches, row by row; the last one opens a colour chooser. */
export const ACCENT_ROWS: Accent[][] = [
  ["pink", "orange", "green", "teal", "blue", "purple"],
  ["red", "amber", "lime", "cyan", "indigo", "custom"],
];

const ACCENT_NAMES: Record<Accent, string> = {
  pink: "Рожевий",
  orange: "Помаранчевий",
  green: "Зелений",
  teal: "Бірюзовий",
  blue: "Синій",
  purple: "Фіолетовий",
  red: "Червоний",
  amber: "Бурштиновий",
  lime: "Лаймовий",
  cyan: "Блакитний",
  indigo: "Індиго",
  custom: "Свій колір",
};
export const accentName = (a: Accent) => ACCENT_NAMES[a];

export interface Settings {
  theme: ThemeMode;
  accent: Accent;
  customAccent: string;
  hwdec: boolean;
  resume: boolean;
  folderPlaylist: boolean;
  autofit: boolean;
  clickPause: boolean;
  volume: number;
}

interface Look {
  accent: string;
  accent_fg: string;
  accent_soft: string;
  palette: Record<string, string>;
}

/** /theme-boot.js reads the theme from here before the first paint (no flash). */
const BOOT_KEY = "videk.theme";
const media = window.matchMedia("(prefers-color-scheme: dark)");

function bootTheme(): ThemeMode {
  try {
    const v = localStorage.getItem(BOOT_KEY);
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
}

class Prefs {
  theme = $state<ThemeMode>(bootTheme());
  accent = $state<Accent>("blue");
  customAccent = $state("#e05a9c");
  hwdec = $state(true);
  resume = $state(true);
  folderPlaylist = $state(true);
  autofit = $state(true);
  clickPause = $state(false);
  /** Swatch colour of each accent on the current theme. */
  palette = $state<Record<string, string>>({});

  systemDark = $state(media.matches);
  dark = $derived(this.theme === "system" ? this.systemDark : this.theme === "dark");

  constructor() {
    media.addEventListener("change", (e) => (this.systemDark = e.matches));
  }

  private take(s: Settings) {
    this.theme = s.theme;
    this.accent = s.accent;
    this.customAccent = s.customAccent;
    this.hwdec = s.hwdec;
    this.resume = s.resume;
    this.folderPlaylist = s.folderPlaylist;
    this.autofit = s.autofit;
    this.clickPause = s.clickPause;
    try {
      localStorage.setItem(BOOT_KEY, s.theme);
    } catch {
      // the next start may flash the other theme
    }
  }

  async init() {
    this.take(await invoke<Settings>("settings_get"));
  }

  /** Changes settings; Rust saves them and applies the mpv ones. */
  async set(patch: Partial<Settings>) {
    this.take(await invoke<Settings>("settings_set", { patch }));
  }

  /** Writes theme + accent to the document and the native window (call from an $effect). */
  applyLook() {
    const dark = this.dark;
    void this.accent;
    void this.customAccent;
    const root = document.documentElement;
    root.dataset.theme = dark ? "dark" : "light";
    invoke<Look>("look", { dark })
      .then((l) => {
        root.style.setProperty("--accent", l.accent);
        root.style.setProperty("--accent-fg", l.accent_fg);
        root.style.setProperty("--accent-soft", l.accent_soft);
        this.palette = l.palette;
      })
      .catch(() => {});
    getCurrentWindow()
      .setTheme(this.theme === "system" ? null : this.theme)
      .catch(() => {});
  }
}

export const prefs = new Prefs();
