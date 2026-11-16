<script lang="ts">
  import Icon from "./Icon.svelte";
  import SeekBar from "./SeekBar.svelte";
  import Volume from "./Volume.svelte";
  import { player } from "../lib/player.svelte";
  import { win } from "../lib/win.svelte";
  import { fmtTime, baseName } from "../lib/format";

  let { onopen, onleave }: { onopen: () => void; onleave: (e: MouseEvent) => void } = $props();

  let remaining = $state(false);
  const long = $derived(player.duration >= 3600);
  const title = $derived(player.title || (player.path ? baseName(player.path) : ""));
  const playlist = $derived(player.playlistCount > 1);
</script>

<header
  class:overlay={win.fullscreen}
  class:hidden={!win.headerShown}
  data-tauri-drag-region
  data-ui
  role="toolbar"
  tabindex="-1"
  onmouseleave={onleave}
>
  <button class="btn logo" onclick={onopen} title="Відкрити файл (Ctrl+O)">
    <Icon name="folder_open" />
  </button>

  {#if player.idle}
    <div class="title grow" data-tauri-drag-region>Videk</div>
  {:else}
    {#if playlist}
      <button class="btn" onclick={() => player.prev()} title="Попередній (PgUp)" disabled={player.playlistPos <= 0}>
        <Icon name="skip_prev" />
      </button>
    {/if}
    <button class="btn play" onclick={() => player.togglePause()} title="Відтворення / пауза (Пробіл)">
      <Icon name={player.pause ? (player.eof ? "replay" : "play") : "pause"} size={22} />
    </button>
    {#if playlist}
      <button
        class="btn"
        onclick={() => player.next()}
        title="Наступний (PgDn)"
        disabled={player.playlistPos >= player.playlistCount - 1}
      >
        <Icon name="skip_next" />
      </button>
    {/if}

    <button class="time" onclick={() => (remaining = !remaining)} title="Показати залишок / тривалість">
      {fmtTime(player.time, long)}<span class="sep">/</span>{remaining
        ? "−" + fmtTime(Math.max(0, player.duration - player.time), long)
        : fmtTime(player.duration, long)}
    </button>

    <SeekBar />
    <Volume />

    <div class="title" data-tauri-drag-region title={title}>{title}</div>
  {/if}

  <button class="btn" onclick={() => win.toggleFullscreen()} title="Повний екран (F, подвійний клік)">
    <Icon name={win.fullscreen ? "fullscreen_exit" : "fullscreen"} size={20} />
  </button>

  <div class="caption">
    <button class="cap" onclick={() => win.minimize()} title="Згорнути" aria-label="Згорнути">
      <Icon name="cap_min" size={10} />
    </button>
    <button
      class="cap"
      onclick={() => win.toggleMaximize()}
      title={win.maximized || win.fullscreen ? "Відновити" : "Розгорнути"}
      aria-label="Розгорнути"
    >
      <Icon name={win.maximized || win.fullscreen ? "cap_restore" : "cap_max"} size={10} />
    </button>
    <button class="cap close" onclick={() => win.close()} title="Закрити" aria-label="Закрити">
      <Icon name="cap_close" size={10} />
    </button>
  </div>
</header>

<style>
  header {
    position: absolute;
    inset: 0 0 auto 0;
    height: var(--header-h);
    display: flex;
    align-items: center;
    gap: 2px;
    padding-left: 4px;
    background: var(--bg-header);
    border-bottom: 1px solid var(--border);
    z-index: 20;
    transition:
      transform 0.18s ease,
      opacity 0.18s ease;
  }
  header.overlay {
    background: color-mix(in srgb, var(--bg-header) 90%, transparent);
    border-bottom-color: transparent;
    box-shadow: var(--shadow);
  }
  header.hidden {
    transform: translateY(-100%);
    opacity: 0;
    pointer-events: none;
  }
  .btn {
    width: 32px;
    height: 28px;
    display: grid;
    place-items: center;
    flex: none;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--fg);
  }
  .btn:hover:not(:disabled) {
    background: var(--hover);
  }
  .btn:active:not(:disabled) {
    background: var(--pressed);
  }
  .btn:disabled {
    opacity: 0.35;
  }
  .btn.play {
    color: var(--accent);
  }
  .logo {
    color: var(--fg-dim);
  }
  .time {
    flex: none;
    padding: 0 8px;
    height: 28px;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--fg-dim);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .time:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .sep {
    margin: 0 4px;
    opacity: 0.5;
  }
  .title {
    flex: 0 1 26%;
    min-width: 28px;
    padding: 0 8px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--fg-dim);
  }
  .title.grow {
    flex: 1 1 auto;
    color: var(--fg);
    font-weight: 600;
  }
  .caption {
    display: flex;
    align-self: stretch;
    flex: none;
    margin-left: 4px;
  }
  .cap {
    width: 46px;
    display: grid;
    place-items: center;
    border: 0;
    background: none;
    color: var(--fg);
  }
  .cap:hover {
    background: var(--hover);
  }
  .cap.close:hover {
    background: var(--close-hover);
    color: var(--close-hover-fg);
  }
  @media (max-width: 560px) {
    .title:not(.grow) {
      display: none;
    }
  }
</style>
