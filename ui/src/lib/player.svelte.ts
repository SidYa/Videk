// The mpv side: mirrored properties (from the `mpv-prop` events of the Rust event loop) and
// the commands the UI sends.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { prefs } from "./prefs.svelte";
import { osd } from "./osd.svelte";

export interface Track {
  id: number;
  type: "video" | "audio" | "sub";
  title?: string;
  lang?: string;
  codec?: string;
  selected?: boolean;
  external?: boolean;
  albumart?: boolean;
  image?: boolean;
  "demux-w"?: number;
  "demux-h"?: number;
  "demux-channel-count"?: number;
  "demux-samplerate"?: number;
}

export interface Chapter {
  title?: string;
  time: number;
}

export const SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 3];

export const VIDEO_EXT = [
  "mkv", "mk3d", "mp4", "m4v", "mov", "avi", "webm", "flv", "f4v", "wmv", "asf", "mpg", "mpeg",
  "m2v", "ts", "m2ts", "mts", "vob", "3gp", "3g2", "ogv", "bik", "bk2", "rm", "rmvb", "divx",
];
export const AUDIO_EXT = [
  "mp3", "flac", "wav", "m4a", "aac", "ogg", "oga", "opus", "wma", "ape", "wv", "mka", "ac3",
  "dts", "alac", "aiff", "aif",
];
export const SUB_EXT = ["srt", "ass", "ssa", "vtt", "sub", "idx", "sup", "smi"];

const num = (v: unknown, d = 0) => (typeof v === "number" && isFinite(v) ? v : d);

class Player {
  pause = $state(true);
  time = $state(0);
  duration = $state(0);
  volume = $state(100);
  mute = $state(false);
  speed = $state(1);
  title = $state("");
  path = $state("");
  idle = $state(true);
  eof = $state(false);
  buffering = $state(false);
  cacheTime = $state(0);
  loop = $state(false);
  tracks = $state<Track[]>([]);
  chapters = $state<Chapter[]>([]);
  playlistCount = $state(0);
  playlistPos = $state(-1);
  dwidth = $state(0);
  dheight = $state(0);
  aid = $state<number | false>(false);
  sid = $state<number | false>(false);
  hwdec = $state("");

  hasVideo = $derived(this.tracks.some((t) => t.type === "video" && t.selected && !t.albumart));
  hasCover = $derived(this.tracks.some((t) => t.type === "video" && t.selected && t.albumart));
  audioTracks = $derived(this.tracks.filter((t) => t.type === "audio"));
  subTracks = $derived(this.tracks.filter((t) => t.type === "sub"));

  private apply(name: string, v: unknown) {
    switch (name) {
      case "pause": this.pause = v === true; break;
      case "time-pos": this.time = num(v); break;
      case "duration": this.duration = num(v); break;
      case "volume": this.volume = num(v, 100); break;
      case "mute": this.mute = v === true; break;
      case "speed": this.speed = num(v, 1); break;
      case "media-title": this.title = typeof v === "string" ? v : ""; break;
      case "path": this.path = typeof v === "string" ? v : ""; break;
      case "idle-active": this.idle = v === true; break;
      case "eof-reached": this.eof = v === true; break;
      case "paused-for-cache": this.buffering = v === true; break;
      case "demuxer-cache-time": this.cacheTime = num(v); break;
      case "loop-file": this.loop = v === "inf" || v === true || (typeof v === "number" && v > 0); break;
      case "track-list": this.tracks = Array.isArray(v) ? (v as Track[]) : []; break;
      case "chapter-list": this.chapters = Array.isArray(v) ? (v as Chapter[]) : []; break;
      case "playlist-count": this.playlistCount = num(v); break;
      case "playlist-pos": this.playlistPos = num(v, -1); break;
      case "dwidth": this.dwidth = num(v); break;
      case "dheight": this.dheight = num(v); break;
      case "aid": this.aid = typeof v === "number" ? v : false; break;
      case "sid": this.sid = typeof v === "number" ? v : false; break;
      case "hwdec-current": this.hwdec = typeof v === "string" ? v : ""; break;
    }
  }

  async init() {
    await listen<{ name: string; value: unknown }>("mpv-prop", (e) =>
      this.apply(e.payload.name, e.payload.value),
    );
    const snap = await invoke<Record<string, unknown>>("mpv_snapshot");
    for (const [k, v] of Object.entries(snap)) this.apply(k, v);
  }

  command(...args: (string | number)[]) {
    return invoke("mpv_command", { args: args.map(String) }).catch((e) => console.warn(e));
  }

  set(name: string, value: string | number | boolean) {
    return invoke("mpv_set", { name, value }).catch((e) => console.warn(e));
  }

  get<T = unknown>(name: string): Promise<T | null> {
    return invoke<T>("mpv_get", { name }).catch(() => null);
  }

  /** Applies the settings that live in mpv. */
  applyPrefs() {
    this.set("hwdec", prefs.hwdec ? "auto-safe" : "no");
    this.set("save-position-on-quit", prefs.resume);
    this.set("resume-playback", prefs.resume);
    this.set("autocreate-playlist", prefs.folderPlaylist ? "filter" : "no");
  }

  open(paths: string[]) {
    paths.forEach((p, i) => {
      if (i === 0 && this.idle === false && prefs.resume) this.command("write-watch-later-config");
      this.command("loadfile", p, i === 0 ? "replace" : "append");
    });
    if (paths.length) this.set("pause", false);
  }

  addSubtitle(path: string) {
    this.command("sub-add", path, "select");
    osd.show("Субтитри додано");
  }

  togglePause() {
    if (this.idle) return;
    if (this.eof && this.pause) {
      this.command("seek", 0, "absolute");
      this.set("pause", false);
      return;
    }
    this.set("pause", !this.pause);
  }

  stop() {
    if (prefs.resume) this.command("write-watch-later-config");
    this.command("stop");
  }

  seek(t: number, exact = true) {
    if (this.idle) return;
    this.command("seek", Math.max(0, t).toFixed(3), exact ? "absolute+exact" : "absolute+keyframes");
  }

  seekBy(d: number) {
    if (this.idle) return;
    this.command("seek", d, "relative+exact");
  }

  setVolume(v: number) {
    const vol = Math.round(Math.min(150, Math.max(0, v)));
    this.set("volume", vol);
    if (this.mute && vol > 0) this.set("mute", false);
    prefs.set("volume", vol);
    osd.show(`Гучність ${vol}%`);
  }

  toggleMute() {
    this.set("mute", !this.mute);
    osd.show(this.mute ? `Гучність ${Math.round(this.volume)}%` : "Звук вимкнено");
  }

  setSpeed(s: number) {
    const v = Math.round(Math.min(4, Math.max(0.1, s)) * 100) / 100;
    this.set("speed", v);
    osd.show(`Швидкість ${v}×`);
  }

  toggleLoop() {
    const on = !this.loop;
    this.set("loop-file", on ? "inf" : "no");
    osd.show(on ? "Повтор файлу: увімк." : "Повтор файлу: вимк.");
  }

  screenshot() {
    if (this.idle) return;
    this.command("screenshot", "video");
    osd.show("Скріншот збережено в «Зображення\\Videk»");
  }

  setAudio(id: number | "no") {
    this.set("aid", id);
  }
  setSub(id: number | "no") {
    this.set("sid", id);
  }

  next() {
    this.command("playlist-next");
  }
  prev() {
    this.command("playlist-prev");
  }
}

export const player = new Player();

export function trackLabel(t: Track): string {
  const parts: string[] = [];
  if (t.title) parts.push(t.title);
  if (t.lang) parts.push(t.lang.toUpperCase());
  if (t.codec) parts.push(t.codec);
  if (t.type === "audio" && t["demux-channel-count"]) parts.push(`${t["demux-channel-count"]} кан.`);
  if (t.external) parts.push("зовнішні");
  return `${t.id}. ${parts.join(" · ") || "Доріжка"}`;
}
