<script lang="ts">
  import {
    ArrowUpDown,
    ChevronDown,
    ChevronUp,
    Download,
    Filter,
    LayoutGrid,
  } from 'lucide-svelte';
  import type { SortState } from '../lib/types';

  let {
    sortState,
    sortDescription,
    filtersVisible = false,
    activeFilterCount = 0,
    onToggleFilters,
  }: {
    sortState: SortState;
    sortDescription?: string;
    filtersVisible?: boolean;
    activeFilterCount?: number;
    onToggleFilters?: () => void;
  } = $props();

  let computedSortLabel = $derived.by(() => {
    if (sortDescription) {
      return sortDescription;
    }

    if (!sortState.column || !sortState.direction) {
      return 'No active sort';
    }

    return `${sortState.column} ${sortState.direction}`;
  });
</script>

<aside class="tool-strip" aria-label="Data tools">
  <div class="tool-section">
    <button
      class="tool-button"
      class:active={filtersVisible || activeFilterCount > 0}
      aria-label="Toggle filters"
      title="Toggle filters"
      onclick={() => onToggleFilters?.()}
      type="button"
    >
      <span class="icon-stack">
        <Filter size={16} />
        {#if !filtersVisible && activeFilterCount > 0}
          <span class="badge">{activeFilterCount}</span>
        {/if}
      </span>
    </button>

    <div class="tool-indicator" role="status" aria-label={computedSortLabel} title={computedSortLabel}>
      {#if sortState.direction === 'asc'}
        <ChevronUp size={16} />
      {:else if sortState.direction === 'desc'}
        <ChevronDown size={16} />
      {:else}
        <ArrowUpDown size={16} />
      {/if}
    </div>
  </div>

  <div class="separator"></div>

  <div class="tool-section tool-section-bottom">
    <button
      class="tool-button tool-button-disabled"
      disabled
      aria-label="Column visibility coming later"
      title="Column visibility coming later"
      type="button"
    >
      <LayoutGrid size={16} />
    </button>

    <button
      class="tool-button tool-button-disabled"
      disabled
      aria-label="More export options coming soon"
      title="More export options coming soon"
      type="button"
    >
      <Download size={16} />
    </button>
  </div>
</aside>

<style>
  .tool-strip {
    width: var(--sk-toolstrip-width);
    min-width: var(--sk-toolstrip-width);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sk-space-md);
    padding: var(--sk-space-md) 0;
    background: var(--sk-glass-sidebar);
    backdrop-filter: var(--sk-glass-sidebar-blur);
    -webkit-backdrop-filter: var(--sk-glass-sidebar-blur);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    box-shadow: var(--sk-shadow-card);
  }

  .tool-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sk-space-sm);
    width: 100%;
  }

  .tool-section-bottom {
    margin-top: auto;
  }

  .separator {
    width: 20px;
    height: 1px;
    background: var(--sk-border);
  }

  .tool-button,
  .tool-indicator {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
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

  .tool-button:hover:not(:disabled),
  .tool-button.active {
    color: var(--sk-text);
    border-color: rgba(0, 169, 165, 0.3);
    box-shadow: var(--sk-shadow-accent);
  }

  .tool-button.active {
    background: rgba(0, 169, 165, 0.14);
  }

  .tool-button-disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .tool-indicator {
    pointer-events: none;
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
</style>
