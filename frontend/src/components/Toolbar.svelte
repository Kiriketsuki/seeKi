<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowUpDown, Download, Filter, LayoutGrid, Search } from 'lucide-svelte';
  import ColumnDropdown from './ColumnDropdown.svelte';
  import type { ColumnInfo } from '../lib/types';

  let {
    filtersVisible = false,
    activeFilterCount = 0,
    searchActive = false,
    searchVisible = false,
    columnsOpen = false,
    columns = [],
    columnVisibility = {},
    hiddenColumnCount = 0,
    hasTable = false,
    sortCount = 0,
    onToggleSearch,
    onToggleFilters,
    onToggleColumns,
    onToggleColumnVisibility,
    onShowAllColumns,
    onCloseColumns,
    onExport,
    onClearSort,
  }: {
    filtersVisible?: boolean;
    activeFilterCount?: number;
    searchActive?: boolean;
    searchVisible?: boolean;
    columnsOpen?: boolean;
    columns: ColumnInfo[];
    columnVisibility: Record<string, boolean>;
    hiddenColumnCount?: number;
    hasTable?: boolean;
    sortCount?: number;
    onToggleSearch?: () => void;
    onToggleFilters?: () => void;
    onToggleColumns?: () => void;
    onToggleColumnVisibility?: (columnName: string, visible: boolean) => void;
    onShowAllColumns?: () => void;
    onCloseColumns?: () => void;
    onExport?: () => void;
    onClearSort?: () => void;
  } = $props();

  let sortTitle = $derived(
    sortCount === 0
      ? ''
      : sortCount === 1
        ? 'Sorted by 1 column — click to clear'
        : `Sorted by ${sortCount} columns — click to clear`
  );

  let searchTitle = $derived(
    searchVisible
      ? 'Close search (Ctrl+K)'
      : searchActive
        ? 'Search active — open search bar (Ctrl+K)'
        : 'Open search (Ctrl+K)'
  );
  let filterTitle = $derived(
    filtersVisible && activeFilterCount > 0
      ? `Close filters — ${activeFilterCount} active (Ctrl+F)`
      : filtersVisible
        ? 'Close filters (Ctrl+F)'
        : activeFilterCount > 0
          ? `Filters active (${activeFilterCount}) — open panel (Ctrl+F)`
          : 'Toggle filters (Ctrl+F)'
  );
  let columnsTitle = $derived(
    columnsOpen
      ? `Close columns panel${hiddenColumnCount > 0 ? ` - ${hiddenColumnCount} hidden` : ''}`
      : `Manage columns${hiddenColumnCount > 0 ? ` - ${hiddenColumnCount} hidden` : ''}`
  );

  let shell: HTMLDivElement | null = null;

  function handleOutsidePointerDown(event: PointerEvent) {
    const target = event.target as Node | null;
    if (!columnsOpen || !shell || !target || shell.contains(target)) return;
    onCloseColumns?.();
  }

  onMount(() => {
    window.addEventListener('pointerdown', handleOutsidePointerDown, true);
    return () => window.removeEventListener('pointerdown', handleOutsidePointerDown, true);
  });
</script>

<div class="toolbar-shell" bind:this={shell}>
  <aside class="toolbar" aria-label="Data tools">
    <button
      type="button"
      class="tool-button"
      class:active={searchActive}
      aria-expanded={searchVisible}
      aria-controls="search-panel"
      aria-label={searchTitle}
      title={searchTitle}
      onclick={() => onToggleSearch?.()}
    >
      <Search size={16} />
    </button>

    <button
      type="button"
      class="tool-button"
      class:active={filtersVisible || activeFilterCount > 0}
      aria-expanded={filtersVisible}
      aria-label={filterTitle}
      title={filterTitle}
      onclick={() => onToggleFilters?.()}
    >
      <span class="icon-stack">
        <Filter size={16} />
        {#if activeFilterCount > 0}
          <span class="badge">{activeFilterCount}</span>
        {/if}
      </span>
    </button>

    {#if sortCount > 0}
      <button
        type="button"
        class="tool-button active"
        aria-label={sortTitle}
        title={sortTitle}
        onclick={() => onClearSort?.()}
      >
        <span class="icon-stack">
          <ArrowUpDown size={16} />
          <span class="badge">{sortCount}</span>
        </span>
      </button>
    {/if}

    <div class="separator" aria-hidden="true"></div>

    <button
      type="button"
      class="tool-button"
      class:active={columnsOpen}
      aria-expanded={columnsOpen}
      aria-controls="columns-panel"
      aria-label={columnsTitle}
      title={columnsTitle}
      onclick={() => onToggleColumns?.()}
    >
      <span class="icon-stack">
        <LayoutGrid size={16} />
        {#if hiddenColumnCount > 0}
          <span class="badge badge--neutral">{hiddenColumnCount}</span>
        {/if}
      </span>
    </button>

    <button
      type="button"
      class="tool-button tool-button--export"
      aria-label={hasTable ? 'Export CSV' : 'Export CSV (select a table first)'}
      title={hasTable ? 'Export CSV' : 'Select a table to export'}
      disabled={!hasTable}
      onclick={() => onExport?.()}
    >
      <Download size={16} />
    </button>
  </aside>

  {#if columnsOpen}
    <ColumnDropdown
      {columns}
      {columnVisibility}
      onToggleColumnVisibility={onToggleColumnVisibility}
      onShowAllColumns={onShowAllColumns}
    />
  {/if}
</div>

<style>
  .toolbar-shell {
    position: relative;
    flex: 0 0 auto;
    align-self: stretch;
    display: flex;
    align-items: stretch;
  }

  .toolbar {
    position: relative;
    z-index: 2;
    width: var(--sk-toolstrip-width);
    min-width: var(--sk-toolstrip-width);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sk-toolbar-gap);
    padding: var(--sk-toolbar-padding-y) 0;
    background: var(--sk-glass-sidebar);
    backdrop-filter: var(--sk-glass-sidebar-blur);
    -webkit-backdrop-filter: var(--sk-glass-sidebar-blur);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    box-shadow: var(--sk-shadow-card);
  }

  .tool-button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--sk-tool-button-size);
    height: var(--sk-tool-button-size);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-button);
    backdrop-filter: var(--sk-glass-button-blur);
    -webkit-backdrop-filter: var(--sk-glass-button-blur);
    color: var(--sk-muted);
  }

  .tool-button {
    cursor: pointer;
  }

  .tool-button:hover,
  .tool-button.active {
    color: var(--sk-text);
    border-color: rgba(0, 169, 165, 0.3);
    box-shadow: var(--sk-shadow-accent);
  }

  .tool-button.active {
    background: rgba(0, 169, 165, 0.14);
  }

  .tool-button--export {
    margin-top: auto;
  }

  .tool-button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    pointer-events: none;
  }

  .separator {
    width: 18px;
    height: 1px;
    background: var(--sk-border);
    margin: var(--sk-space-xs) 0;
  }

  .icon-stack {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .badge {
    position: absolute;
    top: -7px;
    right: -10px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: var(--sk-accent);
    color: white;
    font-size: var(--sk-font-size-xs);
    font-weight: 700;
    line-height: 1;
  }

  .badge--neutral {
    min-width: 18px;
  }

</style>
