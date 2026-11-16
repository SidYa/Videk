<script lang="ts">
  import Icon from "./Icon.svelte";
  import { player } from "../lib/player.svelte";

  let bar: HTMLDivElement;
  let drag = false;
  const MAX = 100; // the slider goes to 100 %; the wheel / keys up to 150 %

  const icon = $derived(
    player.mute || player.volume === 0 ? "volume_off" : player.volume < 50 ? "volume_down" : "volume_up",
  );
  const pct = $derived(player.mute ? 0 : Math.min(100, (player.volume / MAX) * 100));

  function at(x: number) {
    const r = bar.getBoundingClientRect();
    player.setVolume((Math.min(Math.max(x - r.left, 0), r.width) / r.width) * MAX);
  }
  function wheel(e: WheelEvent) {
    e.preventDefault();
    player.setVolume(player.volume + (e.deltaY < 0 ? 5 : -5));
  }
</script>

<div class="vol" onwheel={wheel}>
  <button class="btn" onclick={() => player.toggleMute()} title="Звук (M)">
    <Icon name={icon} />
  </button>
  <div
    class="bar"
    bind:this={bar}
    role="slider"
    tabindex="-1"
    aria-label="Гучність"
    aria-valuemin={0}
    aria-valuemax={150}
    aria-valuenow={player.volume}
    title="Гучність {Math.round(player.volume)}%"
    onpointerdown={(e) => {
      if (e.button !== 0) return;
      bar.setPointerCapture(e.pointerId);
      drag = true;
      at(e.clientX);
    }}
    onpointermove={(e) => drag && at(e.clientX)}
    onpointerup={() => (drag = false)}
    onpointercancel={() => (drag = false)}
  >
    <div class="track"><div class="fill" style:width="{pct}%"></div></div>
  </div>
</div>

<style>
  .vol {
    display: flex;
    align-items: center;
    height: 100%;
    flex: none;
  }
  .bar {
    width: 72px;
    height: 100%;
    display: flex;
    align-items: center;
    margin: 0 6px 0 2px;
    touch-action: none;
  }
  .track {
    width: 100%;
    height: 4px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
  }
  .btn {
    width: 32px;
    height: 28px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--fg);
  }
  .btn:hover {
    background: var(--hover);
  }
  @media (max-width: 640px) {
    .bar {
      display: none;
    }
  }
</style>
