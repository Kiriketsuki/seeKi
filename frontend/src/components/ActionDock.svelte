<script lang="ts">
  import { onMount } from 'svelte';
  import { Download, Filter, LayoutGrid, Search, X } from 'lucide-svelte';
  import ColumnDropdown from './ColumnDropdown.svelte';
  import type { ColumnInfo, SortState } from '../lib/types';

  let {
    searchVisible = false,
    searchTerm = '',
    searchActive = false,
    filtersVisible = false,
    activeFilterCount = 0,
    columnsOpen = false,
    columns = [],
    columnVisibility = {},
    hiddenColumnCount = 0,
    hasTable = false,
    disabled = false,
    sortState = [],
    onToggleSearch,
    onSearchInput,
    onSearchClear,
    onToggleFilters,
    onToggleColumns,
    onToggleColumnVisibility,
    onShowAllColumns,
    onCloseColumns,
    onExport,
    onSearchInputRef,
    onSearchButtonRef,
    onColumnsButtonRef,
    onFilterButtonRef,
  }: {
    searchVisible?: boolean;
    searchTerm?: string;
    searchActive?: boolean;
    filtersVisible?: boolean;
    activeFilterCount?: number;
    columnsOpen?: boolean;
    columns?: ColumnInfo[];
    columnVisibility?: Record<string, boolean>;
    hiddenColumnCount?: number;
    hasTable?: boolean;
    disabled?: boolean;
    sortState?: SortState;
    onToggleSearch?: () => void;
    onSearchInput?: (event: Event) => void;
    onSearchClear?: () => void;
    onToggleFilters?: () => void;
    onToggleColumns?: () => void;
    onToggleColumnVisibility?: (columnName: string, visible: boolean) => void;
    onShowAllColumns?: () => void;
    onCloseColumns?: () => void;
    onExport?: () => void;
    onSearchInputRef?: (node: HTMLInputElement | null) => void;
    onSearchButtonRef?: (node: HTMLButtonElement | null) => void;
    onColumnsButtonRef?: (node: HTMLButtonElement | null) => void;
    onFilterButtonRef?: (node: HTMLButtonElement | null) => void;
  } = $props();

  let shell: HTMLDivElement | null = null;
  let searchInputNode: HTMLInputElement | null = $state(null);
  let searchButtonNode: HTMLButtonElement | null = $state(null);
  let filterButtonNode: HTMLButtonElement | null = $state(null);
  let columnsButtonNode: HTMLButtonElement | null = $state(null);
  let exportButtonNode: HTMLButtonElement | null = $state(null);

  // Roving tabindex: tracks which dock button owns tabindex=0.
  // Indices map to: 0=search, 1=filters, 2=columns, 3=export.
  let activeButtonIndex = $state(0);

  let panelOpen = $derived(searchVisible || columnsOpen);
  let searchQuery = $derived.by(() => searchTerm.trim());
  let controlsDisabled = $derived(disabled || !hasTable);

  let sortDescription = $derived.by(() => {
    const first = sortState[0];
    if (!first) return '';
    const col = columns.find((c) => c.name === first.column);
    const label = col?.display_name ?? first.column;
    const dir = first.direction === 'asc' ? 'ascending' : 'descending';
    return `Sorted by ${label}, ${dir}`;
  });

  let searchTitle = $derived(
    searchVisible
      ? 'Close search (Ctrl+K)'
      : searchActive
        ? 'Search active - open search bar (Ctrl+K)'
        : 'Open search (Ctrl+K)'
  );
  let filterTitle = $derived(
    filtersVisible && activeFilterCount > 0
      ? `Close filters - ${activeFilterCount} active (Ctrl+F)`
      : filtersVisible
        ? 'Close filters (Ctrl+F)'
        : activeFilterCount > 0
          ? `Filters active (${activeFilterCount}) - open panel (Ctrl+F)`
          : 'Toggle filters (Ctrl+F)'
  );
  let columnsTitle = $derived(
    columnsOpen
      ? `Close columns panel${hiddenColumnCount > 0 ? ` - ${hiddenColumnCount} hidden` : ''}`
      : `Manage columns${hiddenColumnCount > 0 ? ` - ${hiddenColumnCount} hidden` : ''}`
  );

  $effect(() => {
    onSearchInputRef?.(searchInputNode);
  });

  $effect(() => {
    onSearchButtonRef?.(searchButtonNode);
  });

  $effect(() => {
    onColumnsButtonRef?.(columnsButtonNode);
  });

  $effect(() => {
    onFilterButtonRef?.(filterButtonNode);
  });

  function handleActionsKeydown(event: KeyboardEvent) {
    const { key } = event;
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(key)) return;
    event.preventDefault();

    const buttons = [searchButtonNode, filterButtonNode, columnsButtonNode, exportButtonNode];
    const total = buttons.length;
    let next: number;

    if (key === 'Home') {
      next = 0;
    } else if (key === 'End') {
      next = total - 1;
    } else if (key === 'ArrowRight' || key === 'ArrowDown') {
      next = (activeButtonIndex + 1) % total;
    } else {
      next = (activeButtonIndex - 1 + total) % total;
    }

    activeButtonIndex = next;
    buttons[next]?.focus();
  }

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

<div class="action-dock" role="toolbar" aria-label="Table actions" bind:this={shell}>
  <span class="sr-only" aria-live="polite" aria-atomic="true">{sortDescription}</span>
  <div class="dock-surface" class:panel-open={panelOpen}>
    <div class="dock-panels" class:panel-open={panelOpen}>
      {#if searchVisible}
        <div id="dock-search-panel" class="dock-panel dock-panel--search" role="region" aria-label="Search rows">
          <div class="search-box">
            <Search size={14} />
            <input
              bind:this={searchInputNode}
              type="text"
              class="search-input"
              class:has-value={searchQuery.length > 0}
              placeholder="Search all text columns..."
              value={searchTerm}
              oninput={onSearchInput}
              disabled={controlsDisabled}
              aria-label="Search rows"
            />
            <button
              type="button"
              class="clear-search"
              aria-label="Clear search"
              disabled={controlsDisabled || searchQuery.length === 0}
              onclick={() => onSearchClear?.()}
            >
              <X size={14} />
            </button>
          </div>
        </div>
      {/if}

      {#if columnsOpen}
        <div id="columns-panel" class="dock-panel dock-panel--columns">
          <ColumnDropdown
            {columns}
            {columnVisibility}
            onToggleColumnVisibility={onToggleColumnVisibility}
            onShowAllColumns={onShowAllColumns}
          />
        </div>
      {/if}
    </div>

    <div class="dock-actions" onkeydown={handleActionsKeydown}>
      <button
        type="button"
        class="dock-button"
        class:active={searchActive}
        data-action="search"
        aria-expanded={searchVisible}
        aria-controls={searchVisible ? 'dock-search-panel' : undefined}
        aria-label={searchTitle}
        title={searchTitle}
        tabindex={activeButtonIndex === 0 ? 0 : -1}
        disabled={controlsDisabled}
        bind:this={searchButtonNode}
        onfocus={() => { activeButtonIndex = 0; }}
        onclick={() => onToggleSearch?.()}
      >
        <Search size={16} />
        <span>Search</span>
      </button>

      <button
        type="button"
        class="dock-button"
        class:active={filtersVisible || activeFilterCount > 0}
        data-action="filters"
        aria-expanded={filtersVisible}
        aria-controls={filtersVisible ? 'data-grid' : undefined}
        aria-label={filterTitle}
        title={filterTitle}
        tabindex={activeButtonIndex === 1 ? 0 : -1}
        disabled={controlsDisabled}
        bind:this={filterButtonNode}
        onfocus={() => { activeButtonIndex = 1; }}
        onclick={() => onToggleFilters?.()}
      >
        <span class="icon-stack">
          <Filter size={16} />
          {#if activeFilterCount > 0}
            <span class="badge">{activeFilterCount}</span>
          {/if}
        </span>
        <span>Filters</span>
      </button>

      <button
        type="button"
        class="dock-button"
        class:active={columnsOpen}
        data-action="columns"
        aria-expanded={columnsOpen}
        aria-controls={columnsOpen ? 'columns-panel' : undefined}
        aria-label={columnsTitle}
        title={columnsTitle}
        tabindex={activeButtonIndex === 2 ? 0 : -1}
        disabled={controlsDisabled}
        bind:this={columnsButtonNode}
        onfocus={() => { activeButtonIndex = 2; }}
        onclick={() => onToggleColumns?.()}
      >
        <span class="icon-stack">
          <LayoutGrid size={16} />
          {#if hiddenColumnCount > 0}
            <span class="badge badge--neutral">{hiddenColumnCount}</span>
          {/if}
        </span>
        <span>Columns</span>
      </button>

      <!-- Export uses !hasTable (not controlsDisabled) intentionally: it opens a server URL
           and doesn't depend on in-flight component state. Disabling during table load
           would block legitimate exports without any functional benefit. -->
      <button
        type="button"
        class="dock-button dock-button--export"
        class:active={false}
        data-action="export"
        aria-label={hasTable ? 'Export CSV' : 'Export CSV (select a table first)'}
        title={hasTable ? 'Export CSV' : 'Select a table to export'}
        tabindex={activeButtonIndex === 3 ? 0 : -1}
        disabled={!hasTable}
        bind:this={exportButtonNode}
        onfocus={() => { activeButtonIndex = 3; }}
        onclick={() => onExport?.()}
      >
        <Download size={16} />
        <span>Export</span>
      </button>
    </div>
  </div>
</div>

<style>
  .action-dock {
    position: absolute;
    left: 50%;
    bottom: var(--sk-dock-inset);
    z-index: 3;
    width: min(var(--sk-dock-panel-width), calc(100% - (2 * var(--sk-dock-inset))));
    transform: translateX(-50%);
    pointer-events: none;
  }

  .dock-surface {
    display: grid;
    grid-template-rows: 0fr auto;
    border: 1px solid var(--sk-border-light);
    border-radius: calc(var(--sk-radius-lg) + 4px);
    background: rgba(255, 255, 255, 0.88);
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    box-shadow: var(--sk-shadow-card);
    overflow: hidden;
    pointer-events: auto;
    transition:
      grid-template-rows 0.18s ease,
      box-shadow 0.18s ease,
      border-color 0.18s ease,
      transform 0.18s ease;
  }

  .dock-surface.panel-open {
    grid-template-rows: 1fr auto;
    border-color: rgba(0, 169, 165, 0.18);
    box-shadow: var(--sk-shadow-card), 0 8px 30px rgba(47, 72, 88, 0.08);
  }

  .dock-panels {
    min-height: 0;
    overflow: hidden;
    opacity: 0;
    transition: opacity 0.16s ease;
  }

  .dock-panels.panel-open {
    opacity: 1;
  }

  .dock-panel {
    padding: var(--sk-space-sm);
    border-bottom: 1px solid var(--sk-border-light);
  }

  .dock-panel--columns {
    padding: 0;
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    width: 100%;
    padding: var(--sk-space-xs) var(--sk-space-md);
    border: 1px solid rgba(0, 169, 165, 0.2);
    border-radius: calc(var(--sk-radius-md) + 2px);
    background: var(--sk-glass-input);
    backdrop-filter: var(--sk-glass-input-blur);
    -webkit-backdrop-filter: var(--sk-glass-input-blur);
    color: var(--sk-muted);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.35);
  }

  .search-input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: none;
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
  }

  .search-input::placeholder {
    color: var(--sk-muted);
  }

  .clear-search {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: var(--sk-glass-button);
    backdrop-filter: var(--sk-glass-button-blur);
    -webkit-backdrop-filter: var(--sk-glass-button-blur);
    color: var(--sk-muted);
    cursor: pointer;
    flex-shrink: 0;
  }

  .clear-search:hover:not(:disabled) {
    color: var(--sk-text);
    border-color: rgba(0, 169, 165, 0.24);
  }

  .clear-search:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .dock-actions {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xs);
  }

  .dock-button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-xs);
    min-height: var(--sk-dock-height);
    padding: 0 var(--sk-space-sm);
    border: 1px solid transparent;
    border-radius: calc(var(--sk-radius-md) + 2px);
    background: transparent;
    color: var(--sk-secondary-strong);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    font-weight: 500;
    cursor: pointer;
    transition:
      background 0.16s ease,
      color 0.16s ease,
      border-color 0.16s ease,
      box-shadow 0.16s ease;
  }

  .dock-button:hover:not(:disabled),
  .dock-button.active {
    color: var(--sk-text);
    background: rgba(255, 255, 255, 0.72);
    border-color: rgba(0, 169, 165, 0.24);
    box-shadow: 0 2px 8px rgba(47, 72, 88, 0.08);
  }

  .dock-button.active {
    background: rgba(0, 169, 165, 0.14);
  }

  .dock-button:focus-visible {
    outline: 2px solid var(--sk-accent);
    outline-offset: 2px;
  }

  .dock-button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
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
    right: -9px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--sk-accent);
    color: #fff;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: var(--sk-font-size-xs);
    font-weight: 700;
    line-height: 1;
  }

  .badge--neutral {
    background: var(--sk-secondary-strong);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (max-width: 720px) {
    .action-dock {
      width: calc(100% - (2 * var(--sk-dock-inset)));
    }

    .dock-button {
      padding: 0 4px;
      font-size: var(--sk-font-size-sm);
    }
  }
</style>
