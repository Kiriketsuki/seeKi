<script lang="ts">
  import type { WizardData, TablePreview } from '../lib/types';

  let {
    wizardData = $bindable(),
    onNext,
    onBack,
  }: {
    wizardData: WizardData;
    onNext: () => void;
    onBack: () => void;
  } = $props();

  type Tab = 'user' | 'system';
  let activeTab: Tab = $state('user');
  let searchTerm: string = $state('');

  // TablePreview now carries a structured `schema` field alongside bare `name`.
  // selected_tables stores "schema.table" qualified keys for unambiguous identification.
  function qualifiedKey(t: TablePreview): string {
    return `${t.schema}.${t.name}`;
  }

  function schemaOfKey(key: string): string {
    const idx = key.indexOf('.');
    return idx === -1 ? 'public' : key.slice(0, idx);
  }

  let userTables = $derived(wizardData.tables.filter((t) => !t.is_system));
  let systemTables = $derived(wizardData.tables.filter((t) => t.is_system));

  // Filter to tables whose schema is currently ticked.
  let schemaFilteredUser = $derived(
    userTables.filter((t) => wizardData.selected_schemas.includes(t.schema)),
  );
  let schemaFilteredSystem = $derived(
    systemTables.filter((t) => wizardData.selected_schemas.includes(t.schema)),
  );

  let visibleTables = $derived(
    (activeTab === 'user' ? schemaFilteredUser : schemaFilteredSystem).filter((t) =>
      t.name.toLowerCase().includes(searchTerm.toLowerCase()),
    ),
  );

  let selectedCount = $derived(wizardData.selected_tables.length);
  let totalSelectable = $derived(schemaFilteredUser.length + schemaFilteredSystem.length);

  // Pre-select user tables in the ticked schemas on first mount.
  $effect(() => {
    if (
      wizardData.tables.length > 0 &&
      wizardData.selected_tables.length === 0 &&
      wizardData.selected_schemas.length > 0
    ) {
      wizardData.selected_tables = schemaFilteredUser.map(qualifiedKey);
    }
  });

  // When a schema is un-ticked, drop its tables from the selection so the
  // final payload doesn't carry references to hidden schemas.
  $effect(() => {
    const allowed = new Set(wizardData.selected_schemas);
    const pruned = wizardData.selected_tables.filter((n) => allowed.has(schemaOfKey(n)));
    if (pruned.length !== wizardData.selected_tables.length) {
      wizardData.selected_tables = pruned;
    }
  });

  function toggleSchema(name: string) {
    if (wizardData.selected_schemas.includes(name)) {
      wizardData.selected_schemas = wizardData.selected_schemas.filter((n) => n !== name);
    } else {
      wizardData.selected_schemas = [...wizardData.selected_schemas, name];
    }
  }

  function formatTableCount(n: number): string {
    return `${n} ${n === 1 ? 'table' : 'tables'}`;
  }

  function toggleTable(key: string) {
    if (wizardData.selected_tables.includes(key)) {
      wizardData.selected_tables = wizardData.selected_tables.filter((n) => n !== key);
    } else {
      wizardData.selected_tables = [...wizardData.selected_tables, key];
    }
  }

  function selectAll() {
    const visible = visibleTables.map(qualifiedKey);
    const existing = wizardData.selected_tables.filter((n) => !visible.includes(n));
    wizardData.selected_tables = [...existing, ...visible];
  }

  function deselectAll() {
    const visible = new Set(visibleTables.map(qualifiedKey));
    wizardData.selected_tables = wizardData.selected_tables.filter((n) => !visible.has(n));
  }

  function formatRows(n: number): string {
    if (n >= 1_000_000) return `~${(n / 1_000_000).toFixed(1)}M rows`;
    if (n >= 1_000) return `~${(n / 1_000).toFixed(0)}K rows`;
    return `${n} rows`;
  }
</script>

<div class="step">
  {#if wizardData.schemas.length > 0}
    <div class="schema-picker" role="group" aria-label="Schemas to include">
      <div class="schema-picker-header">
        <span class="schema-picker-title">Schemas</span>
        <span class="schema-picker-hint">Tick the schemas you want to browse</span>
      </div>
      <div class="schema-list">
        {#each wizardData.schemas as schema (schema.name)}
          {@const checked = wizardData.selected_schemas.includes(schema.name)}
          <label class="schema-row" class:checked>
            <input
              type="checkbox"
              {checked}
              onchange={() => toggleSchema(schema.name)}
              aria-label="Include schema {schema.name}"
            />
            <span class="schema-name">{schema.name}</span>
            <span class="schema-count">{formatTableCount(schema.table_count)}</span>
          </label>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Tabs -->
  <div class="tabs-row">
    <div class="tabs">
      <button
        class="tab"
        class:active={activeTab === 'user'}
        onclick={() => { activeTab = 'user'; searchTerm = ''; }}
        aria-pressed={activeTab === 'user'}
      >
        Your tables ({schemaFilteredUser.length})
      </button>
      <button
        class="tab"
        class:active={activeTab === 'system'}
        onclick={() => { activeTab = 'system'; searchTerm = ''; }}
        aria-pressed={activeTab === 'system'}
      >
        System ({schemaFilteredSystem.length})
      </button>
    </div>
    <span class="selection-count">{selectedCount} of {totalSelectable} selected</span>
  </div>

  <!-- Search -->
  <input
    class="search"
    type="search"
    placeholder="Filter tables…"
    bind:value={searchTerm}
    aria-label="Filter tables"
  />

  <!-- Bulk actions -->
  <div class="bulk-actions">
    <button class="btn-bulk" onclick={selectAll} aria-label="Select all visible tables">Select All</button>
    <button class="btn-bulk" onclick={deselectAll} aria-label="Deselect all visible tables">Deselect All</button>
  </div>

  <!-- Table list -->
  <div class="table-list" role="listbox" aria-multiselectable="true" aria-label="Tables to include">
    {#if visibleTables.length === 0}
      <div class="empty">
        {#if searchTerm}
          No tables match "{searchTerm}"
        {:else if activeTab === 'system'}
          No system tables found
        {:else}
          No tables found
        {/if}
      </div>
    {:else}
      {#each visibleTables as table (qualifiedKey(table))}
        {@const key = qualifiedKey(table)}
        {@const checked = wizardData.selected_tables.includes(key)}
        <label class="table-row" class:checked>
          <input
            type="checkbox"
            {checked}
            onchange={() => toggleTable(key)}
            aria-label="Include table {table.name}"
          />
          <span class="table-name">{table.name}</span>
          <span class="table-rows">{formatRows(table.estimated_rows)}</span>
        </label>
      {/each}
    {/if}
  </div>

  <!-- Actions -->
  <div class="actions">
    <button class="btn-back" onclick={onBack} aria-label="Go back to connection setup">← Back</button>
    <div class="right-actions">
      {#if selectedCount === 0}
        <span class="hint">
          {wizardData.schemas.length > 0 && wizardData.selected_schemas.length === 0
            ? 'Select at least one schema to continue'
            : 'Select at least one table'}
        </span>
      {/if}
      <button
        class="btn-next"
        onclick={onNext}
        disabled={selectedCount === 0}
        aria-label="Proceed to branding"
      >
        Next →
      </button>
    </div>
  </div>
</div>

<style>
  .step { display: flex; flex-direction: column; gap: var(--sk-space-md); }

  .schema-picker {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    border: 1px solid var(--sk-border);
    border-radius: var(--sk-radius-lg);
    background: rgba(255,255,255,0.5);
  }
  .schema-picker-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--sk-space-sm);
  }
  .schema-picker-title {
    font-size: var(--sk-font-size-sm);
    font-weight: 600;
    color: var(--sk-text);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .schema-picker-hint {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
  }
  .schema-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-xs);
  }
  .schema-row {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: 4px var(--sk-space-sm);
    border: 1px solid rgba(47,72,88,0.14);
    border-radius: var(--sk-radius-sm);
    cursor: pointer;
    background: transparent;
    transition: background 0.1s, border-color 0.1s;
  }
  .schema-row:hover { background: rgba(0,169,165,0.04); }
  .schema-row.checked { background: rgba(0,169,165,0.08); border-color: rgba(0,169,165,0.4); }
  .schema-row input[type=checkbox] {
    width: 13px; height: 13px;
    accent-color: var(--sk-accent);
    cursor: pointer;
  }
  .schema-name {
    font-family: var(--sk-font-mono);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
  }
  .schema-count {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
  }

  .tabs-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .tabs {
    display: flex;
    gap: var(--sk-space-xs);
    background: var(--sk-border);
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-xs);
  }
  .tab {
    padding: var(--sk-space-xs) var(--sk-space-md);
    border: none;
    background: transparent;
    border-radius: calc(var(--sk-radius-md) - 2px);
    font-size: var(--sk-font-size-body);
    color: var(--sk-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    white-space: nowrap;
  }
  .tab.active {
    background: white;
    color: var(--sk-text);
    font-weight: 500;
    box-shadow: 0 1px 4px rgba(47,72,88,0.08);
  }
  .selection-count {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
  }

  .search {
    width: 100%;
    box-sizing: border-box;
    background: var(--sk-glass-input);
    border: 1px solid rgba(47,72,88,0.14);
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .search:focus {
    border-color: var(--sk-accent);
    box-shadow: 0 0 0 3px rgba(0,169,165,0.12);
  }

  .bulk-actions { display: flex; gap: var(--sk-space-sm); }
  .btn-bulk {
    padding: var(--sk-space-xs) var(--sk-space-md);
    background: transparent;
    border: 1px solid rgba(47,72,88,0.14);
    border-radius: var(--sk-radius-sm);
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .btn-bulk:hover { background: rgba(47,72,88,0.06); color: var(--sk-text); }

  .table-list {
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--sk-border);
    border-radius: var(--sk-radius-lg);
    background: rgba(255,255,255,0.5);
  }
  .table-row {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    cursor: pointer;
    border-bottom: 1px solid var(--sk-border);
    transition: background 0.1s;
  }
  .table-row:last-child { border-bottom: none; }
  .table-row:hover { background: rgba(0,169,165,0.04); }
  .table-row.checked { background: rgba(0,169,165,0.06); }
  .table-row input[type=checkbox] {
    width: 14px; height: 14px;
    accent-color: var(--sk-accent);
    cursor: pointer;
    flex-shrink: 0;
  }
  .table-name {
    flex: 1;
    font-size: var(--sk-font-size-body);
    font-family: var(--sk-font-mono);
    color: var(--sk-text);
  }
  .table-rows {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    white-space: nowrap;
  }
  .empty {
    padding: var(--sk-space-xl);
    text-align: center;
    color: var(--sk-muted);
    font-size: var(--sk-font-size-body);
  }

  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: var(--sk-space-xs);
  }
  .right-actions { display: flex; align-items: center; gap: var(--sk-space-md); }
  .hint { font-size: var(--sk-font-size-sm); color: var(--sk-muted); }

  .btn-back {
    padding: var(--sk-space-sm) var(--sk-space-lg);
    background: transparent;
    border: 1px solid rgba(47,72,88,0.14);
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .btn-back:hover { background: rgba(47,72,88,0.06); color: var(--sk-text); }

  .btn-next {
    padding: var(--sk-space-sm) var(--sk-space-2xl);
    background: var(--sk-accent);
    color: white;
    border: none;
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-md);
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s, box-shadow 0.15s;
    box-shadow: var(--sk-shadow-accent);
  }
  .btn-next:hover:not(:disabled) { opacity: 0.9; box-shadow: 0 4px 12px rgba(0,169,165,0.3); }
  .btn-next:disabled { opacity: 0.45; cursor: not-allowed; box-shadow: none; }
</style>
