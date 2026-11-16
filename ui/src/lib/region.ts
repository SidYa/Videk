// The WebView only shows (and takes the mouse) where the UI draws: every element marked
// `data-ui` that is on screen. Elsewhere the native video window is visible and gets the
// mouse (its input comes back as `video-mouse` events). Checked every frame, sent to Rust
// only when it changes.
import { invoke } from "@tauri-apps/api/core";

interface R {
  x: number;
  y: number;
  w: number;
  h: number;
  r: number;
}

let last = "";

function collect(): R[] {
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const out: R[] = [];
  for (const el of document.querySelectorAll<HTMLElement>("[data-ui]")) {
    const cs = getComputedStyle(el);
    if (cs.visibility === "hidden" || cs.display === "none" || parseFloat(cs.opacity) < 0.02) continue;
    const b = el.getBoundingClientRect();
    const x0 = Math.max(0, b.left);
    const y0 = Math.max(0, b.top);
    const x1 = Math.min(vw, b.right);
    const y1 = Math.min(vh, b.bottom);
    if (x1 - x0 < 1 || y1 - y0 < 1) continue;
    const r = Math.min(parseFloat(cs.borderTopLeftRadius) || 0, (x1 - x0) / 2, (y1 - y0) / 2);
    out.push({ x: x0, y: y0, w: x1 - x0, h: y1 - y0, r });
  }
  return out;
}

function frame() {
  const rects = collect();
  const key = rects.map((r) => `${r.x | 0},${r.y | 0},${Math.ceil(r.w)},${Math.ceil(r.h)},${r.r | 0}`).join(";");
  if (key !== last) {
    last = key;
    invoke("set_ui_region", { rects }).catch(() => {});
  }
  requestAnimationFrame(frame);
}

export function startRegion() {
  requestAnimationFrame(frame);
}
