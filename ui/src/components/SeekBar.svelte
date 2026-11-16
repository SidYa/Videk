<script lang="ts">
  import { player } from "../lib/player.svelte";
  import { win } from "../lib/win.svelte";
  import { fmtTime } from "../lib/format";

  let bar: HTMLDivElement;
  let hoverX = $state<number | null>(null);
  let dragT = $state<number | null>(null);
  let width = $state(1);
  let lastSeek = 0;

  const dur = $derived(player.duration);
  const shown = $derived(dragT ?? player.time);
  const pct = (t: number) => (dur > 0 ? Math.min(100, Math.max(0, (t / dur) * 100)) : 0);
  const hoverT = $derived(hoverX === null || dur <= 0 ? null : (hoverX / width) * dur);
  const hoverChapter = $derived.by(() => {
    if (hoverT === null) return "";
    let title = "";
    for (const c of player.chapters) if (c.time <= hoverT) title = c.title ?? "";
    return title;
  });

  function timeAt(clientX: number): number {
    const r = bar.getBoundingClientRect();
    return (Math.min(Math.max(clientX - r.left, 0), r.width) / r.width) * dur;
  }

  function down(e: PointerEvent) {
    if (e.button !== 0 || dur <= 0) return;
    bar.setPointerCapture(e.pointerId);
    win.holds++;
    dragT = timeAt(e.clientX);
    player.seek(dragT, false);
    lastSeek = performance.now();
  }

  function move(e: PointerEvent) {
    const r = bar.getBoundingClientRect();
    width = r.width;
    hoverX = Math.min(Math.max(e.clientX - r.left, 0), r.width);
    if (dragT !== null) {
      dragT = timeAt(e.clientX);
      // Keyframe seeks while dragging (fast), at most ~25 per second.
      const now = performance.now();
      if (now - lastSeek > 40) {
        player.seek(dragT, false);
        lastSeek = now;
      }
    }
  }

  function up(e: PointerEvent) {
    if (dragT === null) return;
    player.seek(timeAt(e.clientX), true);
    dragT = null;
    win.holds--;
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    player.seekBy(e.deltaY < 0 ? 5 : -5);
  }
</script>

<div
  class="seek"
  class:active={dragT !== null}
  class:disabled={dur <= 0}
  bind:this={bar}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onpointerleave={() => (hoverX = null)}
  onwheel={wheel}
  role="slider"
  tabindex="-1"
  aria-label="Позиція відтворення"
  aria-valuemin={0}
  aria-valuemax={dur}
  aria-valuenow={shown}
>
  <div class="track">
    <div class="cache" style:width="{pct(player.cacheTime)}%"></div>
    <div class="played" style:width="{pct(shown)}%"></div>
    {#each player.chapters as c (c.time)}
      {#if c.time > 0}<div class="chapter" style:left="{pct(c.time)}%"></div>{/if}
    {/each}
  </div>
  <div class="thumb" style:left="{pct(shown)}%"></div>
  {#if hoverT !== null && hoverX !== null}
    <div class="tip" data-ui style:left="{hoverX}px">
      {fmtTime(hoverT, dur >= 3600)}{#if hoverChapter}<span class="ch">{hoverChapter}</span>{/if}
    </div>
  {/if}
</div>

<style>
  .seek {
    position: relative;
    flex: 1 1 auto;
    min-width: 60px;
    height: 100%;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
  }
  .seek.disabled {
    cursor: default;
    opacity: 0.5;
  }
  .track {
    position: relative;
    width: 100%;
    height: 4px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
    transition: height 0.12s;
  }
  .seek:hover .track,
  .seek.active .track {
    height: 6px;
    border-radius: 3px;
  }
  .cache,
  .played {
    position: absolute;
    inset: 0 auto 0 0;
  }
  .cache {
    background: var(--track-cache);
  }
  .played {
    background: var(--accent);
  }
  .chapter {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: var(--bg-header);
  }
  .thumb {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px var(--bg-header);
    transform: scale(0);
    transition: transform 0.12s;
    pointer-events: none;
  }
  .seek:hover:not(.disabled) .thumb,
  .seek.active .thumb {
    transform: scale(1);
  }
  .tip {
    position: absolute;
    top: calc(100% + 2px);
    transform: translateX(-50%);
    padding: 3px 7px;
    border-radius: 4px;
    background: var(--osd-bg);
    color: var(--on-video);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    pointer-events: none;
    z-index: 5;
  }
  .ch {
    margin-left: 6px;
    color: var(--on-video-dim);
  }
</style>
