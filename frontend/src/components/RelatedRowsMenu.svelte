<script lang="ts">
  import { onMount } from 'svelte';
  import type { ReferenceEntry } from '../lib/types';
  import { referenceEntryKey, referenceEntryLabel } from '../lib/related-rows';

  let {
    entries = [],
    loading = false,
    anchor = { x: 0, y: 0 },
    onSelect,
    onClose,
  }: {
    entries?: ReferenceEntry[];
    loading?: boolean;
    /** Viewport coordinates of the button that opened this popover. */
    anchor?: { x: number; y: number };
    onSelect?: (entry: ReferenceEntry) => void;
    onClose?: () => void;
  } = $props();

  let menuEl: HTMLDivElement | null = $state(null);

  function formatCount(entry: ReferenceEntry): string {
    return entry.capped ? '1000+' : String(entry.count);
  }

  function handlePointerDown(event: PointerEvent) {
    const target = event.target as Node | null;
    if (!menuEl || !target || menuEl.contains(target)) return;
    onClose?.();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose?.();
    }
  }

  onMount(() => {
    window.addEventListener('pointerdown', handlePointerDown, true);
    window.addEventListener('keydown', handleKeydown, true);
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown, true);
      window.removeEventListener('keydown', handleKeydown, true);
    };
  });
</script>

<div
  bind:this={menuEl}
  class="related-rows-menu sk-material sk-material--floating"
  role="menu"
  aria-label="Related rows"
  style={`left: ${anchor.x}px; top: ${anchor.y}px;`}
>
  {#if loading}
    <div class="menu-status" role="status">Loading…</div>
  {:else if entries.length === 0}
    <div class="menu-status">No related rows</div>
  {:else}
    {#each entries as entry (referenceEntryKey(entry))}
      <button
        type="button"
        role="menuitem"
        class="menu-entry"
        onclick={() => onSelect?.(entry)}
      >
        <span class="menu-entry__label">{referenceEntryLabel(entry, entries)}</span>
        <span class="menu-entry__count">{formatCount(entry)}</span>
      </button>
    {/each}
  {/if}
</div>

<style>
  .related-rows-menu {
    position: fixed;
    z-index: var(--sk-z-popover, 1000);
    min-width: 200px;
    max-width: 280px;
    max-height: 260px;
    overflow-y: auto;
    padding: var(--sk-space-xs);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .menu-status {
    padding: var(--sk-space-sm) var(--sk-space-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-muted);
  }

  .menu-entry {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    width: 100%;
    padding: var(--sk-space-xs) var(--sk-space-sm);
    border: none;
    border-radius: var(--sk-radius-sm);
    background: transparent;
    color: var(--sk-text);
    text-align: left;
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    cursor: pointer;
  }

  .menu-entry:hover,
  .menu-entry:focus-visible {
    background: rgba(var(--sk-accent-active-rgb), 0.08);
    outline: none;
  }

  .menu-entry__label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu-entry__count {
    flex: 0 0 auto;
    font-variant-numeric: tabular-nums;
    color: var(--sk-muted);
  }
</style>
