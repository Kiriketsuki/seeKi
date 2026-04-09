<script lang="ts">
  import type { TableInfo } from '../lib/types';

  let {
    tables = [],
    selectedTable = '',
    onSelect,
  }: {
    tables: TableInfo[];
    selectedTable: string;
    onSelect: (tableName: string) => void;
  } = $props();
</script>

<nav class="table-list">
  {#each tables as table}
    <button
      class="table-item"
      class:active={selectedTable === table.name}
      onclick={() => onSelect(table.name)}
    >
      <span class="table-item-name">{table.display_name}</span>
      {#if table.row_count_estimate != null}
        <span class="table-item-count">{table.row_count_estimate.toLocaleString()}</span>
      {/if}
    </button>
  {/each}
</nav>

<style>
  .table-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--sk-space-xs) 0;
  }
  .table-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    width: 100%;
    padding: var(--sk-space-xs) var(--sk-space-md);
    border: none;
    background: none;
    border-radius: var(--sk-radius-sm);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
  }
  .table-item:hover {
    background: var(--sk-border);
  }
  .table-item.active {
    background: var(--sk-accent);
    color: white;
  }
  .table-item-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .table-item-count {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
    flex-shrink: 0;
  }
  .table-item.active .table-item-count {
    color: rgba(255, 255, 255, 0.7);
  }
</style>
