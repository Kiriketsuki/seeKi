<script lang="ts">
  import { X } from 'lucide-svelte';
  import { fetchColumns } from '../lib/api';
  import type { ColumnInfo, TableInfo, ViewAggregate, ViewColumn } from '../lib/types';

  let {
    open = false,
    reachableTables = [],
    value = null,
    onSave,
    onClose,
  }: {
    open?: boolean;
    reachableTables: TableInfo[];
    value?: ViewColumn | null;
    onSave: (column: ViewColumn) => void;
    onClose: () => void;
  } = $props();

  let selectedSchema = $state('');
  let selectedTable = $state('');
  let selectedColumn = $state('');
  let alias = $state('');
  let aggregate = $state<ViewAggregate | ''>('');
  let availableColumns = $state<ColumnInfo[]>([]);
  let loadingColumns = $state(false);
  let error = $state('');
  let lastLoadedKey = $state('');

  function isNumeric(dataType: string): boolean {
    return ['smallint', 'integer', 'bigint', 'real', 'double precision', 'numeric'].includes(dataType);
  }

  function seedFromValue() {
    const fallback = reachableTables[0];
    selectedSchema = value?.source_schema ?? fallback?.schema ?? '';
    selectedTable = value?.source_table ?? fallback?.name ?? '';
    selectedColumn = value?.column_name ?? '';
    alias = value?.alias ?? '';
    aggregate = value?.aggregate ?? '';
    error = '';
    availableColumns = [];
    lastLoadedKey = '';
  }

  async function loadTableColumns(schema: string, table: string) {
    if (!schema || !table) return;
    const key = `${schema}.${table}`;
    if (lastLoadedKey === key) return;

    loadingColumns = true;
    error = '';
    try {
      availableColumns = await fetchColumns(schema, table);
      lastLoadedKey = key;
      if (!availableColumns.some((column) => column.name === selectedColumn)) {
        selectedColumn = availableColumns[0]?.name ?? '';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load columns';
      availableColumns = [];
      selectedColumn = '';
    } finally {
      loadingColumns = false;
    }
  }

  $effect(() => {
    if (!open) return;
    seedFromValue();
  });

  $effect(() => {
    if (!open || !selectedSchema || !selectedTable) return;
    void loadTableColumns(selectedSchema, selectedTable);
  });

  const selectedColumnMeta = $derived.by(
    () => availableColumns.find((column) => column.name === selectedColumn) ?? null,
  );

  const aggregateOptions = $derived.by(() => {
    if (!selectedColumnMeta) return [] as ViewAggregate[];
    const options: ViewAggregate[] = ['COUNT', 'MIN', 'MAX'];
    if (isNumeric(selectedColumnMeta.data_type)) {
      options.unshift('SUM', 'AVG');
    }
    return options;
  });

  function handleTableChange(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    const [schema, table] = value.split('.');
    selectedSchema = schema ?? '';
    selectedTable = table ?? '';
    selectedColumn = '';
    availableColumns = [];
    lastLoadedKey = '';
  }

  function handleSave() {
    if (!selectedSchema || !selectedTable || !selectedColumn) return;
    onSave({
      source_schema: selectedSchema,
      source_table: selectedTable,
      column_name: selectedColumn,
      alias: alias.trim() || null,
      aggregate: aggregate || null,
    });
  }
</script>

{#if open}
  <div
    class="picker-backdrop"
    role="presentation"
    tabindex="0"
    onclick={onClose}
    onkeydown={(event) => event.key === 'Escape' && onClose()}
  >
    <div
      class="picker-card"
      role="dialog"
      aria-modal="true"
      aria-label="Choose a view column"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
    >
      <div class="picker-header">
        <div>
          <h3>Choose column</h3>
          <p>Select a source table, a source column, and an optional aggregate.</p>
        </div>
        <button type="button" class="icon-button" aria-label="Close column picker" onclick={onClose}>
          <X size={16} />
        </button>
      </div>

      <div class="picker-grid">
        <label>
          <span>Source table</span>
          <select value={`${selectedSchema}.${selectedTable}`} onchange={handleTableChange}>
            {#each reachableTables as table (`${table.schema}.${table.name}`)}
              <option value={`${table.schema}.${table.name}`}>{table.schema}.{table.name}</option>
            {/each}
          </select>
        </label>

        <label>
          <span>Source column</span>
          <select bind:value={selectedColumn} disabled={loadingColumns || availableColumns.length === 0}>
            {#each availableColumns as column (column.name)}
              <option value={column.name}>{column.display_name}</option>
            {/each}
          </select>
        </label>

        <label>
          <span>Alias</span>
          <input bind:value={alias} type="text" placeholder="Optional output column name" />
        </label>

        <label>
          <span>Aggregate</span>
          <select bind:value={aggregate} disabled={!selectedColumnMeta}>
            <option value="">None</option>
            {#each aggregateOptions as option}
              <option value={option}>{option}</option>
            {/each}
          </select>
        </label>
      </div>

      {#if selectedColumnMeta}
        <div class="meta">
          <span>{selectedColumnMeta.display_name}</span>
          <span>{selectedColumnMeta.display_type}</span>
        </div>
      {/if}

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="picker-actions">
        <button type="button" class="secondary" onclick={onClose}>Cancel</button>
        <button type="button" class="primary" disabled={!selectedColumn || loadingColumns} onclick={handleSave}>
          Add column
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .picker-backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    background: rgba(27, 41, 46, 0.34);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sk-space-lg);
  }

  .picker-card {
    width: min(680px, 100%);
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.92);
    box-shadow: var(--sk-shadow-card);
    padding: var(--sk-space-xl);
  }

  .picker-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-lg);
  }

  .picker-header h3 {
    margin: 0 0 var(--sk-space-xs);
    font-size: var(--sk-font-size-lg);
  }

  .picker-header p {
    margin: 0;
    color: var(--sk-secondary-strong);
  }

  .picker-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--sk-space-md);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary-strong);
  }

  select,
  input {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.9);
    color: var(--sk-text);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font: inherit;
  }

  .meta {
    display: flex;
    gap: var(--sk-space-sm);
    color: var(--sk-muted);
    font-size: var(--sk-font-size-sm);
  }

  .picker-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sk-space-sm);
  }

  .primary,
  .secondary,
  .icon-button {
    border-radius: var(--sk-radius-md);
    font: inherit;
    cursor: pointer;
  }

  .primary {
    border: none;
    background: var(--sk-accent);
    color: white;
    padding: var(--sk-space-sm) var(--sk-space-lg);
  }

  .secondary {
    border: 1px solid var(--sk-border-light);
    background: transparent;
    color: var(--sk-text);
    padding: var(--sk-space-sm) var(--sk-space-lg);
  }

  .icon-button {
    border: 1px solid var(--sk-border-light);
    background: transparent;
    color: var(--sk-secondary-strong);
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .error {
    margin: 0;
    color: #b91c1c;
  }
</style>
