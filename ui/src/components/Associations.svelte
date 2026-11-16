<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { osd } from "../lib/osd.svelte";

  interface Status {
    registered: string[];
    exe: string | null;
  }

  /** The formats Videk knows (Rust: media.rs). */
  let VIDEO_EXT = $state<string[]>([]);
  let AUDIO_EXT = $state<string[]>([]);
  let selected = $state(new Set<string>());
  let registered = $state(new Set<string>());
  let busy = $state(false);
  let error = $state("");

  const dirty = $derived(
    selected.size !== registered.size || [...selected].some((e) => !registered.has(e)),
  );

  onMount(async () => {
    try {
      const f = await invoke<{ video: string[]; audio: string[] }>("media_formats");
      VIDEO_EXT = f.video;
      AUDIO_EXT = f.audio;
      const s = await invoke<Status>("assoc_status");
      registered = new Set(s.registered);
      selected = new Set(s.registered.length ? s.registered : [...VIDEO_EXT, ...AUDIO_EXT]);
    } catch (e) {
      error = String(e);
    }
  });

  function toggle(e: string) {
    const s = new Set(selected);
    if (s.has(e)) s.delete(e);
    else s.add(e);
    selected = s;
  }

  function setGroup(list: string[], on: boolean) {
    const s = new Set(selected);
    for (const e of list) {
      if (on) s.add(e);
      else s.delete(e);
    }
    selected = s;
  }

  async function apply() {
    busy = true;
    error = "";
    try {
      const s = await invoke<Status>("assoc_apply", {
        selected: [...selected],
      });
      registered = new Set(s.registered);
      osd.show(`Асоціації збережено: ${s.registered.length}`);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function makeDefault() {
    if (dirty || registered.size === 0) await apply();
    invoke("open_default_apps").catch((e) => (error = String(e)));
  }
</script>

{#snippet group(title: string, list: string[])}
  <div class="group">
    <div class="ghead">
      <span>{title}</span>
      <button class="link" onclick={() => setGroup(list, true)}>усі</button>
      <button class="link" onclick={() => setGroup(list, false)}>жодного</button>
    </div>
    <div class="chips">
      {#each list as e (e)}
        <button class="chip" class:on={selected.has(e)} onclick={() => toggle(e)}>{e}</button>
      {/each}
    </div>
  </div>
{/snippet}

<p class="note">
  «Застосувати» реєструє Videk для вибраних типів. Остаточно програму за замовчуванням
  підтверджують у Windows (вимога системи).
</p>

{@render group("Відео", VIDEO_EXT)}
{@render group("Аудіо", AUDIO_EXT)}

{#if error}<div class="err">{error}</div>{/if}

<div class="actions">
  <button class="btn" onclick={apply} disabled={busy || !dirty}>Застосувати</button>
  <button class="btn primary" onclick={makeDefault} disabled={busy}>За замовчуванням у Windows…</button>
</div>

<style>
  .note {
    margin: 0 0 8px;
    color: var(--fg-dim);
    font-size: 11px;
  }
  .group {
    margin-bottom: 8px;
  }
  .ghead {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 4px;
  }
  .ghead span {
    flex: 1;
  }
  .link {
    border: 0;
    padding: 0;
    background: none;
    color: var(--accent);
    font-size: 11px;
  }
  .link:hover {
    text-decoration: underline;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    min-width: 42px;
    padding: 2px 7px;
    border: 1px solid var(--border-soft);
    border-radius: 10px;
    background: var(--bg-input);
    color: var(--fg-dim);
    font-size: 11px;
  }
  .chip:hover {
    color: var(--fg);
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--fg);
  }
  .err {
    margin: 6px 0;
    color: #e5534b;
    font-size: 11px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 10px;
  }
  .btn {
    padding: 5px 12px;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius);
    background: var(--bg-input);
  }
  .btn:hover:not(:disabled) {
    background: var(--hover);
  }
  .btn:disabled {
    opacity: 0.5;
  }
  .btn.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-fg);
  }
  .btn.primary:hover:not(:disabled) {
    background: var(--accent);
    filter: brightness(1.1);
  }
</style>
