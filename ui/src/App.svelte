<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import Header from "./components/Header.svelte";
  import MenuList, { type MenuItem } from "./components/MenuList.svelte";
  import SettingsDialog from "./components/SettingsDialog.svelte";
  import Icon from "./components/Icon.svelte";
  import { player, trackLabel, SPEEDS, VIDEO_EXT, AUDIO_EXT, SUB_EXT } from "./lib/player.svelte";
  import { prefs } from "./lib/prefs.svelte";
  import { win, appWindow } from "./lib/win.svelte";
  import { osd } from "./lib/osd.svelte";
  import { baseName, ext, fmtTime } from "./lib/format";
  import { startRegion } from "./lib/region";

  const HEADER_H = 36;

  let menu = $state<{ x: number; y: number } | null>(null);
  let settingsOpen = $state(false);
  let dropping = $state(false);
  let cursorHidden = $state(false);
  let cursorTimer = 0;
  let hideHeaderTimer = 0;
  let needFit = false;

  const title = $derived(player.title || (player.path ? baseName(player.path) : ""));

  $effect(() => prefs.applyLook());
  $effect(() => win.setTitle(title && !player.idle ? `${title} — Videk` : "Videk"));
  // Menus and dialogs keep the full-screen title bar from sliding away under them.
  $effect(() => {
    if (!menu && !settingsOpen) return;
    untrack(() => win.holds++);
    return () => untrack(() => win.holds--);
  });

  // ---- files ------------------------------------------------------------------------

  async function openFiles() {
    const r = await openDialog({
      multiple: true,
      title: "Відкрити",
      filters: [
        { name: "Медіафайли", extensions: [...VIDEO_EXT, ...AUDIO_EXT] },
        { name: "Відео", extensions: VIDEO_EXT },
        { name: "Аудіо", extensions: AUDIO_EXT },
        { name: "Усі файли", extensions: ["*"] },
      ],
    });
    if (r && r.length) player.open(r);
  }

  async function openSubtitles() {
    const r = await openDialog({
      multiple: true,
      title: "Субтитри",
      filters: [{ name: "Субтитри", extensions: SUB_EXT }, { name: "Усі файли", extensions: ["*"] }],
    });
    for (const p of r ?? []) player.addSubtitle(p);
  }

  function handlePaths(paths: string[]) {
    if (!paths.length) return;
    if (!player.idle && paths.every((p) => SUB_EXT.includes(ext(p)))) {
      for (const p of paths) player.addSubtitle(p);
    } else {
      player.open(paths);
    }
  }

  // ---- startup ----------------------------------------------------------------------

  onMount(() => {
    const unlisten: (() => void)[] = [];
    (async () => {
      await player.init();
      await win.init();
      player.applyPrefs();
      player.set("volume", prefs.volume);

      unlisten.push(
        await listen<string[]>("open-files", (e) => handlePaths(e.payload)),
        await listen<{ kind: string; x: number; y: number; delta: number }>("video-mouse", (e) =>
          videoMouse(e.payload),
        ),
        await listen<{ event: string; reason: number | null; error: number | null }>("mpv-event", async (e) => {
          const ev = e.payload;
          if (ev.event === "start-file") needFit = true;
          if (ev.event === "video-reconfig" && needFit) {
            const w = await player.get<number>("dwidth");
            const h = await player.get<number>("dheight");
            if (w && h) {
              needFit = false;
              if (prefs.autofit) win.fitTo(w, h);
            }
          }
          if (ev.event === "file-loaded") osd.show(title || "", 1500);
          // MPV_END_FILE_REASON_ERROR
          if (ev.event === "end-file" && ev.reason === 4) osd.show("Не вдалося відкрити файл", 2500);
        }),
        await getCurrentWebview().onDragDropEvent((e) => {
          const t = e.payload.type;
          if (t === "enter" || t === "over") dropping = true;
          else if (t === "leave") dropping = false;
          else if (t === "drop") {
            dropping = false;
            handlePaths(e.payload.paths);
          }
        }),
      );

      startRegion();
      handlePaths(await invoke<string[]>("take_pending_files"));
      await appWindow.show();
      await appWindow.setFocus();
    })();
    return () => unlisten.forEach((u) => u());
  });

  // ---- mouse over the video (native window, see lib/region.ts) ------------------------

  function setCursorHidden(h: boolean) {
    if (h === cursorHidden) return;
    cursorHidden = h;
    invoke("set_cursor_hidden", { hidden: h }).catch(() => {});
  }

  function pokeCursor() {
    setCursorHidden(false);
    clearTimeout(cursorTimer);
    cursorTimer = window.setTimeout(() => {
      if (!player.idle && !menu && !settingsOpen) setCursorHidden(true);
    }, 1500);
  }

  function videoMove(y: number) {
    pokeCursor();
    if (!win.fullscreen) return;
    if (y <= 4) {
      clearTimeout(hideHeaderTimer);
      win.revealed = true;
    } else if (win.revealed && y > HEADER_H + 24) {
      win.revealed = false;
    }
  }

  function headerLeave(e: MouseEvent) {
    if (!win.fullscreen) return;
    clearTimeout(hideHeaderTimer);
    if (e.clientY <= 0) return; // left the screen at the top edge: stay
    hideHeaderTimer = window.setTimeout(() => (win.revealed = false), 400);
  }

  /** A click on the video while a menu / dialog is open only closes it. */
  function dismiss(): boolean {
    if (menu) {
      menu = null;
      return true;
    }
    if (settingsOpen) {
      settingsOpen = false;
      return true;
    }
    return false;
  }

  let dismissedOnDown = false;

  function videoMouse(m: { kind: string; x: number; y: number; delta: number }) {
    switch (m.kind) {
      case "move":
        videoMove(m.y);
        break;
      case "down":
        dismissedOnDown = dismiss();
        break;
      case "up":
        if (!dismissedOnDown && prefs.clickPause) player.togglePause();
        dismissedOnDown = false;
        break;
      case "dblclick":
        dismiss();
        win.toggleFullscreen();
        break;
      case "rdown":
        break;
      case "context":
        setCursorHidden(false);
        settingsOpen = false;
        menu = { x: m.x, y: m.y };
        break;
      case "middle":
        player.togglePause();
        break;
      case "back":
        player.seekBy(-5);
        break;
      case "forward":
        player.seekBy(5);
        break;
      case "wheel":
        if (!player.idle) player.setVolume(player.volume + (m.delta > 0 ? 5 : -5));
        break;
    }
  }

  // ---- context menu -----------------------------------------------------------------

  const menuItems = $derived.by((): MenuItem[] => {
    const idle = player.idle;
    const items: MenuItem[] = [
      { label: "Відкрити файл…", icon: "folder_open", shortcut: "Ctrl+O", action: openFiles },
      { label: "Додати субтитри…", icon: "subtitles", disabled: idle, action: openSubtitles },
      { sep: true },
      {
        label: player.pause ? "Відтворити" : "Пауза",
        icon: player.pause ? "play" : "pause",
        shortcut: "Пробіл",
        disabled: idle,
        action: () => player.togglePause(),
      },
      { label: "Зупинити", disabled: idle, action: () => player.stop() },
    ];
    if (player.playlistCount > 1) {
      items.push(
        { label: "Наступний файл", icon: "skip_next", shortcut: "PgDn", disabled: player.playlistPos >= player.playlistCount - 1, action: () => player.next() },
        { label: "Попередній файл", icon: "skip_prev", shortcut: "PgUp", disabled: player.playlistPos <= 0, action: () => player.prev() },
      );
    }
    items.push(
      { sep: true },
      { label: "Повний екран", shortcut: "F", checked: win.fullscreen, action: () => win.toggleFullscreen() },
      { label: "Поверх усіх вікон", shortcut: "T", checked: win.onTop, action: () => win.toggleOnTop() },
      { sep: true },
      {
        label: `Швидкість (${player.speed}×)`,
        icon: "speed",
        disabled: idle,
        submenu: SPEEDS.map((s) => ({
          label: s === 1 ? "Звичайна" : `${s}×`,
          checked: Math.abs(player.speed - s) < 0.001,
          action: () => player.setSpeed(s),
        })),
      },
      {
        label: "Аудіодоріжка",
        icon: "audio",
        disabled: idle || player.audioTracks.length === 0,
        submenu: [
          { label: "Вимкнено", checked: player.aid === false, action: () => player.setAudio("no") },
          ...player.audioTracks.map((t) => ({
            label: trackLabel(t),
            checked: player.aid === t.id,
            action: () => player.setAudio(t.id),
          })),
        ],
      },
      {
        label: "Субтитри",
        icon: "subtitles",
        disabled: idle,
        submenu: [
          { label: "Вимкнено", checked: player.sid === false, action: () => player.setSub("no") },
          ...player.subTracks.map((t) => ({
            label: trackLabel(t),
            checked: player.sid === t.id,
            action: () => player.setSub(t.id),
          })),
          { sep: true },
          { label: "Завантажити з файлу…", icon: "folder_open", action: openSubtitles },
        ],
      },
    );
    if (player.chapters.length > 1) {
      items.push({
        label: "Розділи",
        icon: "movie",
        submenu: player.chapters.map((c, i) => ({
          label: `${fmtTime(c.time)}  ${c.title || `Розділ ${i + 1}`}`,
          action: () => player.seek(c.time),
        })),
      });
    }
    items.push(
      { label: "Повтор файлу", icon: "repeat", shortcut: "L", checked: player.loop || undefined, disabled: idle, action: () => player.toggleLoop() },
      { label: "Скріншот", icon: "camera", shortcut: "S", disabled: idle || !player.hasVideo, action: () => player.screenshot() },
      { sep: true },
      { label: "Налаштування…", icon: "settings", shortcut: "Ctrl+,", action: () => (settingsOpen = true) },
      { label: "Вийти", icon: "logout", action: () => win.close() },
    );
    return items;
  });

  // ---- keyboard ---------------------------------------------------------------------

  function key(e: KeyboardEvent) {
    if (settingsOpen) {
      if (e.key === "Escape") settingsOpen = false;
      return;
    }
    if (menu) {
      if (e.key === "Escape") menu = null;
      return;
    }
    const k = e.key;
    const code = e.code;
    let handled = true;
    if (e.ctrlKey && code === "KeyO") openFiles();
    else if (e.ctrlKey && code === "Comma") settingsOpen = true;
    else if (e.ctrlKey) handled = false;
    else if (k === " " || code === "KeyK" || k === "MediaPlayPause") player.togglePause();
    else if (k === "ArrowRight") player.seekBy(e.shiftKey ? 1 : e.altKey ? 60 : 5);
    else if (k === "ArrowLeft") player.seekBy(e.shiftKey ? -1 : e.altKey ? -60 : -5);
    else if (k === "ArrowUp") player.setVolume(player.volume + 5);
    else if (k === "ArrowDown") player.setVolume(player.volume - 5);
    else if (code === "KeyF" || k === "F11" || (k === "Enter" && e.altKey)) win.toggleFullscreen();
    else if (k === "Escape") {
      if (win.fullscreen) win.setFullscreen(false);
    } else if (code === "KeyM") player.toggleMute();
    else if (code === "KeyL") player.toggleLoop();
    else if (code === "KeyS") player.screenshot();
    else if (code === "KeyT") win.toggleOnTop();
    else if (code === "BracketLeft") player.setSpeed(player.speed - 0.1);
    else if (code === "BracketRight") player.setSpeed(player.speed + 0.1);
    else if (k === "Backspace") player.setSpeed(1);
    else if (code === "Period") player.command("frame-step");
    else if (code === "Comma") player.command("frame-back-step");
    else if (k === "PageDown" || k === "MediaTrackNext") player.next();
    else if (k === "PageUp" || k === "MediaTrackPrevious") player.prev();
    else if (k === "Home") player.seek(0);
    else if (code === "KeyA") player.command("cycle", "audio");
    else if (code === "KeyJ") player.command("cycle", "sub");
    else if (code === "KeyV") player.command("cycle", "sub-visibility");
    else if (/^Digit[0-9]$/.test(code) && player.duration > 0)
      player.seek((Number(code.slice(5)) / 10) * player.duration);
    else handled = false;
    if (handled) e.preventDefault();
  }
</script>

<svelte:window onkeydown={key} />

<Header onopen={openFiles} onleave={headerLeave} />

<!-- The video area. Only the `data-ui` parts are drawn by the WebView (lib/region.ts); the
     mouse over the rest reaches the video window and comes back as `video-mouse`. -->
<div class="stage" class:full={win.fullscreen} class:drop={dropping}>
  {#if player.idle}
    <div class="center idle">
      <img src="/logo.svg" alt="" width="72" height="72" data-ui />
      <button class="open" onclick={openFiles} data-ui>
        <Icon name="folder_open" /> Відкрити файл
      </button>
      <div class="hint" data-ui>або перетягніть файл сюди · ПКМ — меню</div>
    </div>
  {:else if !player.hasVideo && !player.hasCover}
    <div class="center audio">
      <div class="disc" data-ui><Icon name="music_note" size={44} /></div>
      <div class="name" data-ui>{title}</div>
      {#if player.title && player.path && baseName(player.path) !== player.title}
        <div class="hint" data-ui>{baseName(player.path)}</div>
      {/if}
    </div>
  {/if}

  {#if player.buffering}
    <div class="spinner" aria-label="Завантаження" data-ui></div>
  {/if}

  {#if osd.visible && osd.text}
    <div class="osd" data-ui>{osd.text}</div>
  {/if}
</div>

{#if menu}
  <div
    class="catcher"
    role="presentation"
    onpointerdown={(e) => {
      if (e.button === 0) menu = null;
    }}
    oncontextmenu={(e) => {
      e.preventDefault();
      menu = { x: e.clientX, y: e.clientY };
    }}
  ></div>
  <MenuList items={menuItems} x={menu.x} y={menu.y} onpick={() => (menu = null)} />
{/if}

{#if settingsOpen}
  <SettingsDialog onclose={() => (settingsOpen = false)} />
{/if}

<style>
  .stage {
    position: absolute;
    inset: var(--header-h) 0 0 0;
    overflow: hidden;
    color: var(--on-video);
  }
  .stage.full {
    inset: 0;
  }
  .stage.drop::after {
    content: "";
    position: absolute;
    inset: 6px;
    border: 2px dashed var(--accent);
    border-radius: 10px;
    pointer-events: none;
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 16px;
    text-align: center;
    pointer-events: none;
  }
  .center > * {
    pointer-events: auto;
  }
  .idle img {
    opacity: 0.9;
    filter: drop-shadow(0 6px 18px rgba(0, 0, 0, 0.5));
  }
  .open {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 18px;
    border: 0;
    border-radius: 18px;
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 13px;
    font-weight: 600;
  }
  .open:hover {
    filter: brightness(1.1);
  }
  .hint {
    color: var(--on-video-dim);
  }
  .disc {
    width: 96px;
    height: 96px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent);
  }
  .audio .name {
    max-width: 80%;
    font-size: 16px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .osd {
    position: absolute;
    top: 14px;
    left: 50%;
    transform: translateX(-50%);
    max-width: 80%;
    padding: 6px 14px;
    border-radius: 16px;
    background: var(--osd-bg);
    color: var(--on-video);
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }
  .stage.full .osd {
    top: 20px;
  }
  .spinner {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 36px;
    height: 36px;
    margin: -18px 0 0 -18px;
    border: 3px solid rgba(255, 255, 255, 0.2);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    pointer-events: none;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .catcher {
    position: fixed;
    inset: 0;
    z-index: 55;
  }
</style>
