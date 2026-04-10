<script lang="ts">
  import { Check, Square } from 'lucide-svelte';
  import type { ColumnInfo } from '../lib/types';

  let {
    columns = [],
    columnVisibility = {},
    onToggleColumnVisibility,
    onShowAllColumns,
  }: {
    columns: ColumnInfo[];
    columnVisibility: Record<string, boolean>;
    onToggleColumnVisibility?: (columnName: string, visible: boolean) => void;
    onShowAllColumns?: () => void;
  } = $props();

  let panel: HTMLDivElement | null = null;

  function isVisible(column: ColumnInfo): boolean {
    return columnVisibility[column.name] !== false;
  }

  function toggleColumn(column: ColumnInfo) {
    onToggleColumnVisibility?.(column.name, !isVisible(column));
  }

  let hiddenCount = $derived(
    columns.filter((column) => !isVisible(column)).length
  );
</script>

<div class="dropdown" bind:this={panel} role="dialog" aria-label="Column visibility">
  <div class="dropdown-header">
    <div>
      <div class="title">Columns</div>
      <div class="subtitle">
        {hiddenCount > 0 ? `${hiddenCount} hidden` : 'All visible'}
      </div>
    </div>

    <button type="button" class="show-all" onclick={() => onShowAllColumns?.()}>
      Show All
    </button>
  </div>

  <div class="list">
    {#each columns as column}
      <button
        type="button"
        aria-pressed={isVisible(column)}
        class="column-row"
        class:hidden={!isVisible(column)}
        onclick={() => toggleColumn(column)}
      >
        <span class="checkbox" aria-hidden="true">
          {#if isVisible(column)}
            <Check size={12} />
          {:else}
            <Square size={12} />
          {/if}
        </span>
        <span class="column-label">{column.display_name || column.name}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .dropdown {
    position: absolute;
    top: 0;
    left: calc(100% + var(--sk-space-md));
    z-index: 20;
    width: 240px;
    max-height: min(70vh, 560px);
    display: flex;
    flex-direction: column;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.92);
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    box-shadow: var(--sk-shadow-card);
    overflow: hidden;
  }

  .dropdown-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-md);
    border-bottom: 1px solid var(--sk-border-light);
  }

  .title {
    font-size: var(--sk-font-size-md);
    font-weight: 600;
    color: var(--sk-text);
  }

  .subtitle {
    margin-top: 2px;
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary);
  }

  .show-all {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: var(--sk-glass-button);
    color: var(--sk-secondary-strong);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-sm);
    padding: 4px 8px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .show-all:hover {
    color: var(--sk-text);
    border-color: rgba(0, 169, 165, 0.28);
  }

  .list {
    padding: var(--sk-space-xs);
    overflow-y: auto;
  }

  .column-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: 8px 10px;
    border: none;
    border-radius: var(--sk-radius-sm);
    background: transparent;
    color: var(--sk-text);
    text-align: left;
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    cursor: pointer;
  }

  .column-row:hover {
    background: rgba(0, 169, 165, 0.08);
  }

  .column-row.hidden .column-label {
    text-decoration: line-through;
    color: var(--sk-muted);
  }

  .checkbox {
    width: 14px;
    height: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--sk-accent);
  }

  .column-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
