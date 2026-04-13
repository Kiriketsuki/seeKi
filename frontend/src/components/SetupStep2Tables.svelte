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

  let userTables = $derived(wizardData.tables.filter((t) => !t.is_system));
  let systemTables = $derived(wizardData.tables.filter((t) => t.is_system));

  let visibleTables = $derived(
    (activeTab === 'user' ? userTables : systemTables).filter((t) =>
      t.name.toLowerCase().includes(searchTerm.toLowerCase())
    )
  );

  let selectedCount = $derived(wizardData.selected_tables.length);
  let totalSelectable = $derived(userTables.length + systemTables.length);

  // Pre-select user tables on mount
  $effect(() => {
    if (wizardData.tables.length > 0 && wizardData.selected_tables.length === 0) {
      wizardData.selected_tables = userTables.map((t) => t.name);
    }
  });

  function toggleTable(name: string) {
    if (wizardData.selected_tables.includes(name)) {
      wizardData.selected_tables = wizardData.selected_tables.filter((n) => n !== name);
    } else {
      wizardData.selected_tables = [...wizardData.selected_tables, name];
    }
  }

  function selectAll() {
    const visible = visibleTables.map((t) => t.name);
    const existing = wizardData.selected_tables.filter((n) => !visible.includes(n));
    wizardData.selected_tables = [...existing, ...visible];
  }

  function deselectAll() {
    const visible = new Set(visibleTables.map((t) => t.name));
    wizardData.selected_tables = wizardData.selected_tables.filter((n) => !visible.has(n));
  }

  function formatRows(n: number): string {
    if (n >= 1_000_000) return `~${(n / 1_000_000).toFixed(1)}M rows`;
    if (n >= 1_000) return `~${(n / 1_000).toFixed(0)}K rows`;
    return `${n} rows`;
  }
</script>

<div class="step">
  <!-- Tabs -->
  <div class="tabs-row">
    <div class="tabs">
      <button
        class="tab"
        class:active={activeTab === 'user'}
        onclick={() => { activeTab = 'user'; searchTerm = ''; }}
        aria-pressed={activeTab === 'user'}
      >
        Your tables ({userTables.length})
      </button>
      <button
        class="tab"
        class:active={activeTab === 'system'}
        onclick={() => { activeTab = 'system'; searchTerm = ''; }}
        aria-pressed={activeTab === 'system'}
      >
        System ({systemTables.length})
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
      {#each visibleTables as table (table.name)}
        {@const checked = wizardData.selected_tables.includes(table.name)}
        <label class="table-row" class:checked>
          <input
            type="checkbox"
            {checked}
            onchange={() => toggleTable(table.name)}
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
        <span class="hint">Select at least one table</span>
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
    padding: 3px;
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
