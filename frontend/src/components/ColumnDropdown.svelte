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

<div class="dropdown" role="region" aria-label="Column visibility">
  <div class="dropdown-header">
    <div>
      <div class="title">Columns</div>
      <div class="subtitle">
        {hiddenCount > 0 ? `${hiddenCount} hidden` : 'All visible'}
      </div>
    </div>

    <button type="button" class="show-all" disabled={hiddenCount === 0} onclick={() => onShowAllColumns?.()}>
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
  /* Column dropdown panel — mirrors kit.css .sk-col-dropdown */
  .dropdown {
    width: 100%;
    max-height: min(50vh, 320px);
    display: flex;
    flex-direction: column;
    background: transparent;
    overflow: hidden;
  }

  /* Header: title + "Show All" action — mirrors .sk-col-dropdown-head */
  .dropdown-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    border-bottom: 1px solid var(--sk-border-light);
  }

  /* Title: uppercase eyebrow label — mirrors .sk-col-dropdown-title */
  .title {
    font-size: var(--sk-font-size-sm);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--sk-secondary-strong);
  }

  .subtitle {
    margin-top: var(--sk-space-xs);
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
  }

  /* "Show All" button — dark teal ink for AA contrast, mirrors .sk-col-showall */
  .show-all {
    border: none;
    background: none;
    color: var(--sk-data-ink);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    cursor: pointer;
    flex-shrink: 0;
  }

  .show-all:hover:not(:disabled) {
    background: var(--sk-active-tint-soft);
    border-radius: var(--sk-radius-sm);
  }

  .show-all:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Scrollable column list — mirrors .sk-col-list */
  .list {
    max-height: 220px;
    overflow-y: auto;
    padding: var(--sk-space-xs);
    display: flex;
    flex-direction: column;
  }

  /* Individual column row — mirrors .sk-col-item */
  .column-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
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

  /* hover: teal-6% wash — mirrors .sk-col-item:hover */
  .column-row:hover {
    background: rgba(var(--sk-accent-active-rgb), 0.06);
  }

  .column-row.hidden .column-label {
    text-decoration: line-through;
    color: var(--sk-muted);
  }

  /* Checkbox icon — dark teal for AA contrast — mirrors .sk-col-item input accent-color */
  .checkbox {
    width: 14px;
    height: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--sk-data-ink);
  }

  /* Column name — mono type label for the data type variant would go here */
  .column-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
