/** Seconds as h:mm:ss / m:ss. */
export function fmtTime(sec: number, withHours = false): string {
  if (!isFinite(sec) || sec < 0) sec = 0;
  const s = Math.floor(sec % 60);
  const m = Math.floor((sec / 60) % 60);
  const h = Math.floor(sec / 3600);
  const ss = String(s).padStart(2, "0");
  if (h > 0 || withHours) return `${h}:${String(m).padStart(2, "0")}:${ss}`;
  return `${m}:${ss}`;
}

/** The file name of a path or URL. */
export function baseName(path: string): string {
  const p = path.replace(/[\\/]+$/, "");
  return p.slice(Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\")) + 1) || path;
}

export function ext(path: string): string {
  const b = baseName(path);
  const i = b.lastIndexOf(".");
  return i < 0 ? "" : b.slice(i + 1).toLowerCase();
}
