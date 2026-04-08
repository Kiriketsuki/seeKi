<script lang="ts">
  import { Search, LayoutGrid, Download } from 'lucide-svelte';

  let {
    tableName = '',
    rowCount = 0,
  }: {
    tableName: string;
    rowCount: number;
  } = $props();
</script>

<div class="toolbar">
  <div class="table-info">
    <span class="table-name">{tableName}</span>
    <span class="row-count">{rowCount.toLocaleString()} rows</span>
  </div>

  <div class="actions">
    <div class="search-wrapper">
      <Search size={14} />
      <input
        type="text"
        placeholder="Search all columns..."
        disabled
        class="search-input"
      />
    </div>

    <button class="btn btn-default" disabled>
      <LayoutGrid size={14} />
      <span>Columns</span>
    </button>

    <button class="btn btn-accent" disabled>
      <Download size={14} />
      <span>Export</span>
    </button>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-space-md) var(--sk-space-2xl);
    background: var(--sk-glass-toolbar);
    backdrop-filter: var(--sk-glass-toolbar-blur);
    -webkit-backdrop-filter: var(--sk-glass-toolbar-blur);
    border-bottom: 1px solid var(--sk-border-light);
    gap: var(--sk-space-lg);
  }

  .table-info {
    display: flex;
    align-items: baseline;
    gap: var(--sk-space-sm);
    min-width: 0;
  }

  .table-name {
    font-size: var(--sk-font-size-xl);
    font-weight: 600;
    color: var(--sk-text);
    white-space: nowrap;
  }

  .row-count {
    font-size: 11px;
    color: var(--sk-secondary);
    white-space: nowrap;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  .search-wrapper {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-xs) var(--sk-space-md);
    background: var(--sk-glass-input);
    backdrop-filter: var(--sk-glass-input-blur);
    -webkit-backdrop-filter: var(--sk-glass-input-blur);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    color: var(--sk-muted);
  }

  .search-input {
    border: none;
    background: none;
    outline: none;
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
    width: 160px;
  }

  .search-input::placeholder {
    color: var(--sk-muted);
  }

  .search-input:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xs) var(--sk-space-md);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    cursor: pointer;
    white-space: nowrap;
  }

  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .btn-default {
    background: var(--sk-glass-button);
    backdrop-filter: var(--sk-glass-button-blur);
    -webkit-backdrop-filter: var(--sk-glass-button-blur);
    color: var(--sk-text);
  }

  .btn-accent {
    background: var(--sk-accent);
    color: white;
    border-color: transparent;
  }
</style>
