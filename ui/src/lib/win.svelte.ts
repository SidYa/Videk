// The window: full screen, maximized, always on top, and the title bar shown over the video
// in full screen. (Fitting the window to a new video is done in Rust: media.rs.)
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

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
}

export const win = new Win();
export { appWindow };
