<script lang="ts">
  import Icon from "./Icon.svelte";
  import Associations from "./Associations.svelte";
  import { prefs, ACCENT_ROWS, accentName, type ThemeMode, type Accent } from "../lib/prefs.svelte";
  import { player } from "../lib/player.svelte";

  let { onclose }: { onclose: () => void } = $props();
  let colorInput: HTMLInputElement;

  const THEMES: [ThemeMode, string][] = [
    ["system", "Системна"],
    ["light", "Світла"],
    ["dark", "Темна"],
  ];

  type Toggle = "hwdec" | "resume" | "folderPlaylist" | "autofit" | "clickPause";
  const TOGGLES: [Toggle, string, string][] = [
    ["hwdec", "Апаратне декодування", "Декодування відео на GPU (hwdec=auto-safe). Вимкніть, якщо бачите артефакти."],
    ["resume", "Пам'ятати позицію", "Продовжувати перегляд з місця, де зупинилися."],
    ["folderPlaylist", "Файли з папки як плейлист", "Наступні / попередні відео й аудіо тієї ж папки."],
    ["autofit", "Розмір вікна під відео", "Під час відкриття вікно підлаштовується під розмір кадру (до 80% екрана)."],
    ["clickPause", "Пауза клацанням по відео", "Одинарний клік ЛКМ ставить на паузу."],
  ];

  /** Swatch colour on the current theme (Rust: accent.rs). */
  function swatch(a: Accent): string {
    return a === "custom" ? prefs.customAccent : (prefs.palette[a] ?? "transparent");
  }

  function toggle(key: Toggle) {
    prefs.set({ [key]: !prefs[key] });
  }
</script>

<div
  class="backdrop"
  role="presentation"
  onpointerdown={(e) => e.target === e.currentTarget && onclose()}
  oncontextmenu={(e) => e.stopPropagation()}
>
  <div class="dialog" data-ui role="dialog" aria-modal="true" aria-label="Налаштування">
    <div class="head">
      <Icon name="settings" />
      <span>Налаштування</span>
      <span class="ver">
        Videk 0.1.0 · libmpv{player.hwdec && player.hwdec !== "no" ? ` · ${player.hwdec}` : ""}
      </span>
      <button class="x" onclick={onclose} aria-label="Закрити"><Icon name="cap_close" size={10} /></button>
    </div>

    <div class="body">
      <div class="col">
      <section>
        <h3>Вигляд</h3>
        <div class="row">
          <span class="name">Тема</span>
          <div class="seg">
            {#each THEMES as [t, label] (t)}
              <button class:on={prefs.theme === t} onclick={() => prefs.set({ theme: t })}>{label}</button>
            {/each}
          </div>
        </div>
        <div class="row top">
          <span class="name">Акцентний колір</span>
          <div class="swatches">
            {#each ACCENT_ROWS as row, r (r)}
              <div class="srow">
                {#each row as a (a)}
                  <button
                    class="sw"
                    class:on={prefs.accent === a}
                    class:custom={a === "custom"}
                    style:--c={swatch(a)}
                    title={accentName(a)}
                    aria-label={accentName(a)}
                    onclick={() => (a === "custom" ? colorInput.click() : prefs.set({ accent: a }))}
                  ></button>
                {/each}
              </div>
            {/each}
            <input
              bind:this={colorInput}
              type="color"
              class="hidden-color"
              value={prefs.customAccent}
              oninput={(e) => prefs.set({ customAccent: (e.currentTarget as HTMLInputElement).value, accent: "custom" })}
            />
          </div>
        </div>
      </section>

      <section>
        <h3>Відтворення</h3>
        {#each TOGGLES as [key, label, hint] (key)}
          <label class="row toggle" title={hint}>
            <span class="name">{label}</span>
            <input type="checkbox" checked={prefs[key]} onchange={() => toggle(key)} />
            <span class="switch" aria-hidden="true"></span>
          </label>
        {/each}
      </section>
      </div>

      <div class="col">
        <section>
          <h3>Асоціації файлів</h3>
          <Associations />
        </section>
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: var(--backdrop);
  }
  .dialog {
    width: min(800px, calc(100vw - 24px));
    max-height: calc(100vh - 24px);
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-soft);
    border-radius: 10px;
    background: var(--bg-panel);
    color: var(--fg);
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 8px 10px 14px;
    font-size: 13px;
    font-weight: 600;
    color: var(--fg);
  }
  .head :global(.icon) {
    color: var(--accent);
  }
  .head span {
    flex: 1;
  }
  .head .ver {
    flex: none;
    margin-right: 4px;
    font-size: 11px;
    font-weight: 400;
    color: var(--fg-faint);
  }
  .x {
    width: 30px;
    height: 26px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius);
    background: none;
  }
  .x:hover {
    background: var(--close-hover);
    color: var(--close-hover-fg);
  }
  /* Two columns, everything visible at once (no scrolling unless the window is tiny). */
  .body {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 0 20px;
    overflow-y: auto;
    padding: 0 14px 14px;
  }
  .col + .col {
    padding-left: 20px;
    border-left: 1px solid var(--border-soft);
  }
  @media (max-width: 700px) {
    .body {
      grid-template-columns: 1fr;
    }
    .col + .col {
      padding-left: 0;
      border-left: 0;
      border-top: 1px solid var(--border-soft);
      margin-top: 8px;
    }
  }
  section + section {
    margin-top: 4px;
    border-top: 1px solid var(--border-soft);
  }
  h3 {
    margin: 8px 0 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-dim);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 30px;
    padding: 2px 0;
  }
  .row.top {
    align-items: flex-start;
  }
  .name {
    color: var(--fg);
  }
  .seg {
    display: flex;
    padding: 2px;
    border-radius: 7px;
    background: var(--bg-input);
    border: 1px solid var(--border-soft);
  }
  .seg button {
    border: 0;
    padding: 4px 12px;
    border-radius: 5px;
    background: none;
    color: var(--fg-dim);
  }
  .seg button.on {
    background: var(--accent);
    color: var(--accent-fg);
  }
  .seg button:not(.on):hover {
    background: var(--hover);
    color: var(--fg);
  }
  .swatches {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .srow {
    display: flex;
    gap: 6px;
  }
  .sw {
    width: 24px;
    height: 24px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 50%;
    background: var(--c) content-box;
    outline: 1px solid var(--border-soft);
    outline-offset: -1px;
  }
  .sw.custom {
    background:
      conic-gradient(from 0deg, #ef5350, #f2b632, #a5c94b, #2bb3a3, #4b9be8, #a77be8, #e8607a, #ef5350)
      content-box;
  }
  .sw.on {
    border-color: var(--bg-panel);
    outline: 2px solid var(--c);
    outline-offset: 0;
  }
  .hidden-color {
    position: absolute;
    width: 0;
    height: 0;
    opacity: 0;
    pointer-events: none;
  }
  .toggle {
    position: relative;
  }
  .toggle input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .switch {
    position: relative;
    flex: none;
    width: 34px;
    height: 18px;
    border-radius: 9px;
    background: var(--track);
    transition: background 0.15s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--fg-dim);
    transition:
      transform 0.15s,
      background 0.15s;
  }
  .toggle input:checked + .switch {
    background: var(--accent);
  }
  .toggle input:checked + .switch::after {
    transform: translateX(16px);
    background: var(--accent-fg);
  }
  .toggle input:focus-visible + .switch {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
