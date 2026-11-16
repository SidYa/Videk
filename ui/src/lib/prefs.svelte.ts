// Settings (context menu > Налаштування). Each one is applied at once and remembered in
// localStorage (WebView2 profile). Theme and accent work like in Farba.
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

/** Accent colours per theme ([dark, light]), as in Farba. */
export const ACCENTS: Record<Exclude<Accent, "custom">, [string, string]> = {
  pink: ["#e8607a", "#c2185b"],
  orange: ["#f0913a", "#d0600a"],
  green: ["#5cb860", "#2e7d32"],
  teal: ["#2bb3a3", "#00897b"],
  blue: ["#4b9be8", "#1f6fd1"],
  purple: ["#a77be8", "#6a3fc8"],
  red: ["#ef5350", "#c62828"],
  amber: ["#f2b632", "#a86200"],
  lime: ["#a5c94b", "#5a8a12"],
  cyan: ["#37b9d8", "#00789e"],
  indigo: ["#7f8cf0", "#3f51b5"],
};

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

function load<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem("videk." + key);
    return v === null ? fallback : (JSON.parse(v) as T);
  } catch {
    return fallback;
  }
}

function save(key: string, value: unknown) {
  try {
    localStorage.setItem("videk." + key, JSON.stringify(value));
  } catch {
    // not remembered
  }
}

// ---- colour helpers (Farba) ----------------------------------------------------------

/** Backgrounds the accent must stand out from, per theme. */
const ACCENT_BACKS = { dark: ["#2b2b2b", "#2f2f2f", "#1f1f1f"], light: ["#f6f6f6", "#ffffff"] };
const ACCENT_CONTRAST = 3;

function rgbOf(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [n >> 16, (n >> 8) & 255, n & 255];
}

function hexOf(c: number[]): string {
  return (
    "#" +
    c.map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, "0")).join("")
  );
}

function luminance(c: number[]): number {
  const l = c.map((v) => {
    const s = v / 255;
    return s <= 0.04045 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * l[0]! + 0.7152 * l[1]! + 0.0722 * l[2]!;
}

function contrast(a: number[], b: number[]): number {
  const [x, y] = [luminance(a), luminance(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}

/** The accent adapted to a theme: lightened/darkened (same hue) until it stands out. */
export function accentFor(hex: string, dark: boolean): string {
  const c = rgbOf(hex);
  const backs = ACCENT_BACKS[dark ? "dark" : "light"].map(rgbOf);
  const ok = (v: number[]) => backs.every((b) => contrast(v, b) >= ACCENT_CONTRAST);
  if (ok(c)) return hex;
  const to = dark ? 255 : 0;
  for (let t = 0.05; t <= 1; t += 0.05) {
    const v = c.map((x) => x + (to - x) * t);
    if (ok(v)) return hexOf(v);
  }
  return dark ? "#ffffff" : "#000000";
}

function accentText(hex: string): string {
  const c = rgbOf(hex);
  return contrast(c, [255, 255, 255]) >= contrast(c, [26, 26, 26]) ? "#ffffff" : "#1a1a1a";
}

// ---- prefs ---------------------------------------------------------------------------

const media = window.matchMedia("(prefers-color-scheme: dark)");

function readTheme(): ThemeMode {
  try {
    const v = localStorage.getItem("videk.theme");
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
}

class Prefs {
  theme = $state<ThemeMode>(readTheme());
  systemDark = $state(media.matches);
  dark = $derived(this.theme === "system" ? this.systemDark : this.theme === "dark");

  accent = $state<Accent>(load("accent", "blue"));
  customAccent = $state<string>(load("accent-custom", "#e05a9c"));

  /** Hardware decoding (mpv hwdec=auto-safe). */
  hwdec = $state<boolean>(load("hwdec", true));
  /** Remember the position of each file and resume from it. */
  resume = $state<boolean>(load("resume", true));
  /** Play the other files of the folder as a playlist. */
  folderPlaylist = $state<boolean>(load("folder-playlist", true));
  /** Size the window to the video when a file opens. */
  autofit = $state<boolean>(load("autofit", true));
  /** A click on the video pauses / plays. */
  clickPause = $state<boolean>(load("click-pause", false));
  /** Volume of the last session. */
  volume = $state<number>(load("volume", 100));

  constructor() {
    media.addEventListener("change", (e) => (this.systemDark = e.matches));
  }

  accentHex(a: Accent = this.accent): string {
    if (a === "custom") return this.customAccent;
    return ACCENTS[a][this.dark ? 0 : 1];
  }

  setTheme(t: ThemeMode) {
    this.theme = t;
    try {
      localStorage.setItem("videk.theme", t);
    } catch {
      // not remembered
    }
  }
  setAccent(a: Accent) {
    this.accent = a;
    save("accent", a);
  }
  setCustomAccent(hex: string) {
    this.customAccent = hex;
    this.accent = "custom";
    save("accent-custom", hex);
    save("accent", "custom");
  }
  set<K extends "hwdec" | "resume" | "folderPlaylist" | "autofit" | "clickPause" | "volume">(
    key: K,
    value: Prefs[K],
  ) {
    (this as Prefs)[key] = value;
    const storeKey = { hwdec: "hwdec", resume: "resume", folderPlaylist: "folder-playlist", autofit: "autofit", clickPause: "click-pause", volume: "volume" }[key];
    save(storeKey, value);
  }

  /** Writes theme + accent to the document and the native window (call from an $effect). */
  applyLook() {
    const root = document.documentElement;
    root.dataset.theme = this.dark ? "dark" : "light";
    const acc = accentFor(this.accentHex(), this.dark);
    const [r, g, b] = rgbOf(acc);
    root.style.setProperty("--accent", acc);
    root.style.setProperty("--accent-fg", accentText(acc));
    root.style.setProperty("--accent-soft", `rgba(${r}, ${g}, ${b}, ${this.dark ? 0.28 : 0.2})`);
    getCurrentWindow()
      .setTheme(this.theme === "system" ? null : this.theme)
      .catch(() => {});
  }
}

export const prefs = new Prefs();
