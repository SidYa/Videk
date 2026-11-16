// The window: full screen, maximized, always on top, the title bar shown over the video in
// full screen, and fitting the window to the video size.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, currentMonitor, PhysicalPosition, PhysicalSize } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
const HEADER_H = 36;

class Win {
  fullscreen = $state(false);
  maximized = $state(false);
  onTop = $state(false);
  /** Full screen: the title bar slid in (cursor at the top edge). */
  revealed = $state(false);
  /** Something holds the title bar open (menu, seek drag, dialog). */
  holds = $state(0);

  headerShown = $derived(!this.fullscreen || this.revealed || this.holds > 0);

  async init() {
    const sync = async () => {
      this.maximized = await appWindow.isMaximized();
      const fs = await appWindow.isFullscreen();
      if (fs !== this.fullscreen) {
        this.fullscreen = fs;
        this.revealed = false;
      }
    };
    await sync();
    await appWindow.onResized(sync);
  }

  async setFullscreen(on: boolean) {
    this.fullscreen = on;
    this.revealed = false;
    await invoke("set_fullscreen", { on }).catch((e) => console.warn(e));
  }

  toggleFullscreen() {
    return this.setFullscreen(!this.fullscreen);
  }

  async toggleOnTop() {
    this.onTop = !this.onTop;
    await appWindow.setAlwaysOnTop(this.onTop);
  }

  minimize() {
    appWindow.minimize();
  }
  toggleMaximize() {
    if (this.fullscreen) return this.setFullscreen(false);
    appWindow.toggleMaximize();
  }
  close() {
    appWindow.close();
  }
  startDrag() {
    appWindow.startDragging();
  }
  setTitle(t: string) {
    appWindow.setTitle(t).catch(() => {});
  }

  /** Sizes the window so the video shows at 1:1 (at most 80% of the screen), keeping the
   *  window centre where it was. */
  async fitTo(vw: number, vh: number) {
    if (this.fullscreen || this.maximized || vw <= 0 || vh <= 0) return;
    const mon = await currentMonitor();
    if (!mon) return;
    const area = mon.workArea ?? { position: mon.position, size: mon.size };
    const scale = mon.scaleFactor;
    const header = Math.round(HEADER_H * scale);
    const maxW = area.size.width * 0.8;
    const maxH = area.size.height * 0.8 - header;
    const k = Math.min(1, maxW / vw, maxH / vh);
    const minW = Math.round(420 * scale);
    let w = Math.max(minW, Math.round(vw * k));
    let h = Math.round((w / vw) * vh) + header;
    h = Math.max(Math.round(240 * scale), h);
    w = Math.min(w, area.size.width);
    h = Math.min(h, area.size.height);

    const pos = await appWindow.outerPosition();
    const size = await appWindow.outerSize();
    const cx = pos.x + size.width / 2;
    const cy = pos.y + size.height / 2;
    const ax = area.position.x;
    const ay = area.position.y;
    const x = Math.round(Math.min(Math.max(cx - w / 2, ax), ax + area.size.width - w));
    const y = Math.round(Math.min(Math.max(cy - h / 2, ay), ay + area.size.height - h));
    await appWindow.setSize(new PhysicalSize(w, h));
    await appWindow.setPosition(new PhysicalPosition(x, y));
  }
}

export const win = new Win();
export { appWindow };
