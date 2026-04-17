<script lang="ts">
  import { ArrowDown, ArrowUp, Pencil, Plus, Trash2, X } from 'lucide-svelte';
  import { createView, fetchFkPath, previewView } from '../lib/api';
  import ColumnPickerPopover from './ColumnPickerPopover.svelte';
  import type {
    QueryResult,
    SavedViewSummary,
    TableInfo,
    ViewColumn,
    ViewDraft,
  } from '../lib/types';

  let {
    tables = [],
    initialDraft = null,
    sourceLabel = '',
    onCancel,
    onSaved,
  }: {
    tables: TableInfo[];
    initialDraft: ViewDraft | null;
    sourceLabel?: string;
    onCancel: () => void;
    onSaved: (view: SavedViewSummary) => Promise<void> | void;
  } = $props();

  let name = $state('');
  let baseSchema = $state('');
  let baseTable = $state('');
  let columns = $state<ViewColumn[]>([]);
  let filters = $state<Record<string, string>>({});
  let preview = $state<QueryResult | null>(null);
  let previewLoading = $state(false);
  let reachableTables = $state<TableInfo[]>([]);
  let reachableLoading = $state(false);
  let pickerOpen = $state(false);
  let pickerIndex = $state<number | null>(null);
  let pickerValue = $state<ViewColumn | null>(null);
  let error = $state('');
  let saving = $state(false);
  let previewTimer: ReturnType<typeof setTimeout> | null = null;
  let previewRequestId = 0;
  let reachabilityRequestId = 0;
  let seedKey = $state('');

  function computeOutputNames(items: ViewColumn[]): string[] {
    const bareCounts = new Map<string, number>();
    for (const item of items) {
      if (!item.alias && !item.aggregate) {
        bareCounts.set(item.column_name, (bareCounts.get(item.column_name) ?? 0) + 1);
      }
    }

    return items.map((item) => {
      if (item.alias?.trim()) return item.alias.trim();
      if (item.aggregate) {
        return `${item.aggregate.toLowerCase()}_${item.source_table}__${item.column_name}`;
      }
      if ((bareCounts.get(item.column_name) ?? 0) > 1) {
        return `${item.source_table}__${item.column_name}`;
      }
      return item.column_name;
    });
  }

  function describeColumn(item: ViewColumn, outputName: string): string {
    const aggregate = item.aggregate ? `${item.aggregate}(` : '';
    const suffix = item.aggregate ? ')' : '';
    return `${aggregate}${item.source_table}.${item.column_name}${suffix} → ${outputName}`;
  }

  function resetToDraft(nextDraft: ViewDraft | null) {
    name = nextDraft?.name ?? '';
    baseSchema = nextDraft?.base_schema ?? tables[0]?.schema ?? '';
    baseTable = nextDraft?.base_table ?? tables[0]?.name ?? '';
    columns = nextDraft?.columns ? nextDraft.columns.map((column) => ({ ...column })) : [];
    filters = nextDraft?.filters ? { ...nextDraft.filters } : {};
    preview = null;
    error = '';
    pickerOpen = false;
    pickerIndex = null;
    pickerValue = null;
  }

  $effect(() => {
    const nextSeedKey = JSON.stringify(initialDraft ?? { empty: true });
    if (seedKey === nextSeedKey) return;
    seedKey = nextSeedKey;
    resetToDraft(initialDraft);
  });

  const outputNames = $derived.by(() => computeOutputNames(columns));
  const columnRows = $derived.by(() =>
    columns.map((column, index) => ({
      column,
      index,
      outputName: outputNames[index],
    })),
  );
  const filterableColumns = $derived.by(() =>
    columnRows.filter(({ column }) => !column.aggregate),
  );

  $effect(() => {
    const allowedKeys = new Set(filterableColumns.map(({ outputName }) => outputName));
    const nextFilters = Object.fromEntries(
      Object.entries(filters).filter(([key, value]) => allowedKeys.has(key) && value.trim().length > 0),
    );
    if (JSON.stringify(nextFilters) !== JSON.stringify(filters)) {
      filters = nextFilters;
    }
  });

  async function computeReachableTables(schema: string, table: string) {
    if (!schema || !table) {
      reachableTables = [];
      return;
    }
    const myRequest = ++reachabilityRequestId;
    reachableLoading = true;
    try {
      const sameSchemaTables = tables.filter((candidate) => candidate.schema === schema);
      const reachable = await Promise.all(
        sameSchemaTables.map(async (candidate) => {
          if (candidate.schema === schema && candidate.name === table) return true;
          try {
            const path = await fetchFkPath(schema, table, candidate.schema, candidate.name);
            return path.length > 0;
          } catch {
            return false;
          }
        }),
      );
      if (myRequest !== reachabilityRequestId) return;
      reachableTables = sameSchemaTables.filter((_, index) => reachable[index]);
    } finally {
      if (myRequest === reachabilityRequestId) {
        reachableLoading = false;
      }
    }
  }

  $effect(() => {
    if (!baseSchema || !baseTable) return;
    void computeReachableTables(baseSchema, baseTable);
  });

  $effect(() => {
    if (previewTimer) clearTimeout(previewTimer);
    if (!baseSchema || !baseTable || columns.length === 0) {
      preview = null;
      previewLoading = false;
      return;
    }

    const myRequest = ++previewRequestId;
    previewLoading = true;
    previewTimer = setTimeout(async () => {
      try {
        const result = await previewView({
          base_schema: baseSchema,
          base_table: baseTable,
          columns,
          filters,
        });
        if (myRequest !== previewRequestId) return;
        preview = result;
        error = '';
      } catch (err) {
        if (myRequest !== previewRequestId) return;
        error = err instanceof Error ? err.message : 'Preview failed';
        preview = null;
      } finally {
        if (myRequest === previewRequestId) {
          previewLoading = false;
        }
      }
    }, 300);

    return () => {
      if (previewTimer) clearTimeout(previewTimer);
    };
  });

  function handleBaseTableChange(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    const [schema, table] = value.split('.');
    baseSchema = schema ?? '';
    baseTable = table ?? '';
    columns = [];
    filters = {};
    preview = null;
  }

  function openPicker(index: number | null) {
    pickerIndex = index;
    pickerValue = index == null
      ? {
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: '',
          alias: null,
          aggregate: null,
        }
      : { ...columns[index] };
    pickerOpen = true;
  }

  function handlePickerSave(nextColumn: ViewColumn) {
    if (pickerIndex == null) {
      columns = [...columns, nextColumn];
    } else {
      columns = columns.map((column, index) => (index === pickerIndex ? nextColumn : column));
    }
    pickerOpen = false;
    pickerIndex = null;
    pickerValue = null;
  }

  function removeColumn(index: number) {
    columns = columns.filter((_, currentIndex) => currentIndex !== index);
  }

  function moveColumn(index: number, direction: -1 | 1) {
    const nextIndex = index + direction;
    if (nextIndex < 0 || nextIndex >= columns.length) return;
    const nextColumns = [...columns];
    const [moved] = nextColumns.splice(index, 1);
    nextColumns.splice(nextIndex, 0, moved);
    columns = nextColumns;
  }

  function updateFilter(outputName: string, value: string) {
    filters = {
      ...filters,
      [outputName]: value,
    };
  }

  async function handleSave() {
    const trimmedName = name.trim();
    if (!trimmedName) {
      error = 'Saved view name must not be empty';
      return;
    }
    if (!baseSchema || !baseTable) {
      error = 'Choose a base table first';
      return;
    }
    if (columns.length === 0) {
      error = 'Select at least one output column';
      return;
    }

    saving = true;
    error = '';
    try {
      const saved = await createView({
        name: trimmedName,
        base_schema: baseSchema,
        base_table: baseTable,
        columns,
        filters,
      });
      onSaved(saved);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to save view';
    } finally {
      saving = false;
    }
  }
</script>

<div class="builder">
  <div class="builder-header">
    <div>
      <h2>Custom view builder</h2>
      <p>Choose a base table, add joined columns, and preview the result without writing SQL.</p>
      {#if sourceLabel}
        <span class="source-pill">Started from {sourceLabel}</span>
      {/if}
    </div>
    <button type="button" class="icon-btn" aria-label="Close builder" onclick={onCancel}>
      <X size={16} />
    </button>
  </div>

  <div class="builder-grid">
    <section class="panel">
      <div class="panel-heading">
        <h3>Definition</h3>
      </div>

      <label class="field">
        <span>Saved view name</span>
        <input bind:value={name} type="text" placeholder="Orders with customers" />
      </label>

      <label class="field">
        <span>Base table</span>
        <select value={`${baseSchema}.${baseTable}`} onchange={handleBaseTableChange}>
          {#each tables as table (`${table.schema}.${table.name}`)}
            <option value={`${table.schema}.${table.name}`}>{table.schema}.{table.name}</option>
          {/each}
        </select>
      </label>

      <div class="panel-subheader">
        <div>
          <h4>Output columns</h4>
          <p>{reachableLoading ? 'Finding FK-reachable tables…' : 'Each selection becomes an exposed output column.'}</p>
        </div>
        <button type="button" class="secondary" disabled={!baseSchema || !baseTable || reachableLoading} onclick={() => openPicker(null)}>
          <Plus size={14} />
          <span>Add column</span>
        </button>
      </div>

      {#if columnRows.length > 0}
        <div class="column-list">
          {#each columnRows as { column, index, outputName } (index)}
            <div class="column-row">
              <div class="column-copy">
                <strong>{outputName}</strong>
                <span>{describeColumn(column, outputName)}</span>
              </div>
              <div class="column-actions">
                <button type="button" class="icon-btn" aria-label={`Move ${outputName} up`} onclick={() => moveColumn(index, -1)} disabled={index === 0}>
                  <ArrowUp size={14} />
                </button>
                <button type="button" class="icon-btn" aria-label={`Move ${outputName} down`} onclick={() => moveColumn(index, 1)} disabled={index === columnRows.length - 1}>
                  <ArrowDown size={14} />
                </button>
                <button type="button" class="icon-btn" aria-label={`Edit ${outputName}`} onclick={() => openPicker(index)}>
                  <Pencil size={14} />
                </button>
                <button type="button" class="icon-btn" aria-label={`Remove ${outputName}`} onclick={() => removeColumn(index)}>
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="empty-card">No columns selected yet. Add at least one column to preview and save the view.</div>
      {/if}

      <div class="panel-subheader">
        <div>
          <h4>Definition filters</h4>
          <p>These filters are stored with the view and applied before preview and browsing.</p>
        </div>
      </div>

      {#if filterableColumns.length > 0}
        <div class="filter-list">
          {#each filterableColumns as { outputName }}
            <label class="field" for={`filter-${outputName}`}>
              <span>{outputName}</span>
              <input
                id={`filter-${outputName}`}
                type="text"
                value={filters[outputName] ?? ''}
                oninput={(event) => updateFilter(outputName, (event.currentTarget as HTMLInputElement).value)}
                placeholder={`Filter ${outputName}`}
              />
            </label>
          {/each}
        </div>
      {:else}
        <div class="empty-card">Definition filters are available for non-aggregate output columns.</div>
      {/if}

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="builder-actions">
        <button type="button" class="secondary" onclick={onCancel}>Cancel</button>
        <button type="button" class="primary" disabled={saving} onclick={handleSave}>
          {saving ? 'Saving…' : 'Save view'}
        </button>
      </div>
    </section>

    <section class="panel preview-panel">
      <div class="panel-heading">
        <div>
          <h3>Live preview</h3>
          <p>Preview requests are debounced and capped at 100 rows.</p>
        </div>
        {#if previewLoading}
          <span class="loading-chip">Refreshing…</span>
        {/if}
      </div>

      {#if preview && preview.columns.length > 0}
        <div class="preview-table-wrap">
          <table class="preview-table">
            <thead>
              <tr>
                {#each preview.columns as column (column.name)}
                  <th>{column.display_name}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each preview.rows as row, rowIndex (rowIndex)}
                <tr>
                  {#each preview.columns as column (column.name)}
                    <td>{String(row[column.name] ?? '')}</td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="empty-card preview-empty">
          {#if columns.length === 0}
            Add columns to preview the custom view.
          {:else if previewLoading}
            Running preview…
          {:else}
            No preview rows yet.
          {/if}
        </div>
      {/if}
    </section>
  </div>

  <ColumnPickerPopover
    open={pickerOpen}
    reachableTables={reachableTables}
    value={pickerValue}
    onSave={handlePickerSave}
    onClose={() => {
      pickerOpen = false;
      pickerIndex = null;
      pickerValue = null;
    }}
  />
</div>

<style>
  .builder {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
    padding: var(--sk-space-lg) var(--sk-space-2xl);
    overflow: auto;
  }

  .builder-header,
  .panel-heading,
  .panel-subheader {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-md);
  }

  .builder-header h2,
  .panel-heading h3,
  .panel-subheader h4 {
    margin: 0 0 var(--sk-space-xs);
  }

  .builder-header p,
  .panel-heading p,
  .panel-subheader p {
    margin: 0;
    color: var(--sk-secondary-strong);
  }

  .source-pill {
    margin-top: var(--sk-space-sm);
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    background: rgba(0, 169, 165, 0.08);
    color: var(--sk-accent);
    padding: 4px 10px;
    font-size: var(--sk-font-size-xs);
    font-weight: 600;
  }

  .builder-grid {
    display: grid;
    grid-template-columns: minmax(340px, 420px) minmax(0, 1fr);
    gap: var(--sk-space-lg);
    min-height: 0;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-md);
    min-height: 0;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.68);
    box-shadow: var(--sk-shadow-card);
    padding: var(--sk-space-xl);
  }

  .preview-panel {
    min-width: 0;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary-strong);
  }

  input,
  select {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.85);
    color: var(--sk-text);
    font: inherit;
    padding: var(--sk-space-sm) var(--sk-space-md);
  }

  .column-list,
  .filter-list {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
  }

  .column-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-md);
    padding: var(--sk-space-sm);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.78);
  }

  .column-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .column-copy span {
    color: var(--sk-muted);
    font-size: var(--sk-font-size-sm);
  }

  .column-actions,
  .builder-actions {
    display: flex;
    align-items: center;
    gap: var(--sk-space-xs);
  }

  .builder-actions {
    justify-content: flex-end;
    margin-top: auto;
  }

  .primary,
  .secondary,
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
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

  .icon-btn {
    width: 32px;
    height: 32px;
    border: 1px solid var(--sk-border-light);
    background: rgba(255, 255, 255, 0.7);
    color: var(--sk-secondary-strong);
    flex-shrink: 0;
  }

  .empty-card {
    border: 1px dashed var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-lg);
    color: var(--sk-muted);
    background: rgba(255, 255, 255, 0.48);
  }

  .preview-table-wrap {
    overflow: auto;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.88);
  }

  .preview-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--sk-font-size-sm);
  }

  .preview-table th,
  .preview-table td {
    padding: 10px 12px;
    border-bottom: 1px solid var(--sk-border-light);
    text-align: left;
    white-space: nowrap;
  }

  .preview-table th {
    position: sticky;
    top: 0;
    background: rgba(245, 240, 235, 0.92);
  }

  .loading-chip {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
  }

  .error {
    margin: 0;
    color: #b91c1c;
  }

  @media (max-width: 980px) {
    .builder-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
