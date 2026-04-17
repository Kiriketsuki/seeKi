<script lang="ts">
  import { Search } from 'lucide-svelte';
  import type { TableInfo } from '../lib/types';

  let {
    tables = [],
    selectedSchema = '',
    selectedTable = '',
    onSelect,
  }: {
    tables: TableInfo[];
    selectedSchema: string;
    selectedTable: string;
    onSelect: (table: TableInfo) => void;
  } = $props();

  // Bare table names that appear in more than one selected schema — these must
  // always be shown qualified, even when the schema is `public`.
  let collidingNames = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const t of tables) counts.set(t.name, (counts.get(t.name) ?? 0) + 1);
    const out = new Set<string>();
    for (const [name, n] of counts) if (n > 1) out.add(name);
    return out;
  });

  function prettyLabel(t: TableInfo): string {
    if (collidingNames.has(t.name)) return `${t.schema}.${t.name}`;
    return t.schema === 'public' ? t.name : `${t.schema}.${t.name}`;
  }

  let search = $state('');
  let filteredTables = $derived.by(() => {
    const query = search.trim().toLowerCase();
    if (!query) {
      return tables;
    }

    return tables.filter(
      (table) =>
        prettyLabel(table).toLowerCase().includes(query) ||
        table.display_name.toLowerCase().includes(query),
    );
  });
</script>

<nav class="table-list">
  <div class="section-header">
    <div class="section-title">Tables</div>
    <div class="section-subtitle">{tables.length} available</div>
  </div>

  <div class="table-search">
    <Search size={14} />
    <input
      bind:value={search}
      class="table-search-input"
      type="search"
      placeholder="Search tables"
      aria-label="Search tables"
      spellcheck="false"
    />
  </div>

  {#if filteredTables.length > 0}
    {#each filteredTables as table (`${table.schema}.${table.name}`)}
      <button
        type="button"
        class="table-item"
        class:active={selectedSchema === table.schema && selectedTable === table.name}
        onclick={() => onSelect(table)}
        title={`${table.schema}.${table.name}`}
      >
        <span class="table-item-name">{prettyLabel(table)}</span>
        {#if table.row_count_estimate != null}
          <span class="table-item-count">{table.row_count_estimate.toLocaleString()}</span>
        {/if}
      </button>
    {/each}
  {:else}
    <div class="empty-state">No tables match “{search.trim()}”.</div>
  {/if}
</nav>

<style>
  .table-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--sk-space-xs) 0;
  }

  .section-header {
    padding: 0 var(--sk-space-md) var(--sk-space-xs);
  }

  .section-title {
    font-size: var(--sk-font-size-sm);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--sk-secondary-strong);
  }

  .section-subtitle {
    margin-top: 2px;
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
  }

  .table-search {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    margin-bottom: var(--sk-space-sm);
    padding: 0 var(--sk-space-md);
    color: var(--sk-muted);
  }

  .table-search-input {
    width: 100%;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.72);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    padding: 6px 10px;
    outline: none;
  }

  .table-search-input:focus {
    border-color: rgba(0, 169, 165, 0.4);
    box-shadow: 0 0 0 2px rgba(0, 169, 165, 0.12);
  }

  .table-search-input::placeholder {
    color: var(--sk-muted);
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

  .empty-state {
    padding: var(--sk-space-sm) var(--sk-space-md);
    color: var(--sk-muted);
    font-size: var(--sk-font-size-body);
  }
</style>
