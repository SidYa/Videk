<script lang="ts" module>
  export type MenuItem =
    | {
        sep?: false;
        label: string;
        icon?: string;
        shortcut?: string;
        checked?: boolean;
        disabled?: boolean;
        action?: () => void;
        submenu?: MenuItem[];
      }
    | { sep: true };
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";
  import MenuList from "./MenuList.svelte";
  import { tick } from "svelte";

  // x, y: where the menu wants its top-left corner; `alt`: the x to use when it does not
  // fit to the right (a submenu then opens to the left of its parent).
  let {
    items,
    x,
    y,
    alt,
    onpick,
  }: { items: MenuItem[]; x: number; y: number; alt?: number; onpick: () => void } = $props();

  let el: HTMLDivElement;
  let left = $state(0);
  let top = $state(0);
  let ready = $state(false);
  let openIndex = $state(-1);
  let subAnchor = $state<{ x: number; y: number; alt: number } | null>(null);
  let hoverTimer = 0;

  $effect(() => {
    void x;
    void y;
    ready = false;
    tick().then(() => {
      const r = el.getBoundingClientRect();
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      let l = x;
      if (l + r.width > vw - 4) l = alt !== undefined ? alt - r.width : vw - 4 - r.width;
      left = Math.max(4, l);
      top = Math.max(4, Math.min(y, vh - 4 - r.height));
      ready = true;
    });
  });

  function enter(i: number, e: MouseEvent) {
    clearTimeout(hoverTimer);
    const item = items[i]!;
    const target = e.currentTarget as HTMLElement;
    hoverTimer = window.setTimeout(() => {
      if (!item.sep && item.submenu && !item.disabled) {
        const r = target.getBoundingClientRect();
        subAnchor = { x: r.right - 2, y: r.top - 5, alt: r.left + 2 };
        openIndex = i;
      } else {
        openIndex = -1;
      }
    }, item.sep || !item.submenu ? 120 : 60);
  }

  function click(i: number, e: MouseEvent) {
    const item = items[i]!;
    if (item.sep || item.disabled) return;
    if (item.submenu) {
      clearTimeout(hoverTimer);
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      subAnchor = { x: r.right - 2, y: r.top - 5, alt: r.left + 2 };
      openIndex = i;
      return;
    }
    item.action?.();
    onpick();
  }

  const sub = $derived.by(() => {
    const it = openIndex >= 0 ? items[openIndex] : undefined;
    return it && !it.sep ? it.submenu : undefined;
  });
</script>

<div
  class="menu"
  data-ui
  role="menu"
  tabindex="-1"
  bind:this={el}
  style:left="{left}px"
  style:top="{top}px"
  style:visibility={ready ? "visible" : "hidden"}
  oncontextmenu={(e) => e.preventDefault()}
>
  {#each items as item, i (i)}
    {#if item.sep}
      <div class="sep"></div>
    {:else}
      <div
        class="item"
        class:open={openIndex === i}
        role={item.checked !== undefined ? "menuitemcheckbox" : "menuitem"}
        aria-checked={item.checked}
        aria-disabled={item.disabled}
        tabindex="-1"
        onmouseenter={(e) => enter(i, e)}
        onclick={(e) => click(i, e)}
        onkeydown={() => {}}
      >
        <span class="ic">
          {#if item.checked}<Icon name="check" size={16} />{:else if item.icon}<Icon name={item.icon} size={16} />{/if}
        </span>
        <span class="label">{item.label}</span>
        {#if item.shortcut}<span class="key">{item.shortcut}</span>{/if}
        {#if item.submenu}<span class="arrow"><Icon name="chevron_right" size={16} /></span>{/if}
      </div>
    {/if}
  {/each}
</div>

{#if sub && subAnchor}
  <MenuList items={sub} x={subAnchor.x} y={subAnchor.y} alt={subAnchor.alt} {onpick} />
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 60;
    min-width: 200px;
    max-width: 380px;
    max-height: calc(100vh - 8px);
    overflow-y: auto;
    padding: 4px;
    border: 1px solid var(--border-soft);
    border-radius: 8px;
    background: var(--bg-popup);
    color: var(--fg);
    box-shadow: var(--shadow);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 8px 0 6px;
    border-radius: 5px;
    white-space: nowrap;
  }
  .item:hover:not([aria-disabled="true"]),
  .item.open {
    background: var(--hover);
  }
  .item[aria-disabled="true"] {
    color: var(--fg-faint);
  }
  .ic {
    width: 16px;
    display: grid;
    place-items: center;
    color: var(--accent);
    flex: none;
  }
  .item[aria-disabled="true"] .ic {
    color: inherit;
  }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .key {
    color: var(--fg-dim);
    margin-left: 18px;
    font-size: 11px;
  }
  .arrow {
    display: grid;
    color: var(--fg-dim);
    margin-right: -4px;
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border-soft);
  }
</style>
