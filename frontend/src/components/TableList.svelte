<script lang="ts">
  import { Pencil, Search } from 'lucide-svelte';
  import type { TableInfo } from '../lib/types';

  const MAX_DISPLAY_NAME_LENGTH = 64;

  let {
    tables = [],
    selectedSchema = '',
    selectedTable = '',
    onSelect,
    onRename,
    showHeader = true,
  }: {
    tables: TableInfo[];
    selectedSchema: string;
    selectedTable: string;
    onSelect: (table: TableInfo) => void;
    onRename: (table: TableInfo, displayName: string) => void;
    showHeader?: boolean;
  } = $props();

  let collidingNames = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const table of tables) {
      counts.set(table.name, (counts.get(table.name) ?? 0) + 1);
    }

    const collisions = new Set<string>();
    for (const [name, count] of counts) {
      if (count > 1) {
        collisions.add(name);
      }
    }

    return collisions;
  });

  function prettyLabel(table: TableInfo): string {
    if (collidingNames.has(table.name)) {
      return `${table.schema}.${table.name}`;
    }

    return table.schema === 'public' ? table.name : `${table.schema}.${table.name}`;
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

  function tableKey(table: TableInfo): string {
    return `${table.schema}.${table.name}`;
  }

  let editingKey = $state<string | null>(null);
  let editValue = $state('');
  let editInputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (editingKey && editInputEl) {
      editInputEl.focus();
      editInputEl.select();
    }
  });

  function startEdit(table: TableInfo) {
    editingKey = tableKey(table);
    editValue = table.display_name;
  }

  function cancelEdit() {
    editingKey = null;
    editValue = '';
  }

  function commitEdit(table: TableInfo) {
    if (editingKey !== tableKey(table)) return;
    const trimmed = editValue.trim().slice(0, MAX_DISPLAY_NAME_LENGTH);
    editingKey = null;
    editValue = '';
    if (trimmed === table.display_name) return;
    onRename(table, trimmed);
  }

  function handleEditKeydown(event: KeyboardEvent, table: TableInfo) {
    if (event.key === 'Enter') {
      event.preventDefault();
      commitEdit(table);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      cancelEdit();
    }
  }
</script>

<nav class="table-list" class:headerless={!showHeader} data-testid="table-list">
  {#if showHeader}
    <div class="section-header">
      <div class="section-title">Tables</div>
      <div class="section-subtitle">{tables.length} available</div>
    </div>
  {/if}

  <label class="panel-search table-search" data-testid="table-list-search">
    <Search size={14} />
    <input
      bind:value={search}
      class="panel-search-input table-search-input"
      type="search"
      placeholder="Search tables"
      aria-label="Search tables"
      spellcheck="false"
      data-testid="table-search-input"
    />
  </label>

  <div class="list-items" data-testid="table-list-items">
    {#if filteredTables.length > 0}
      {#each filteredTables as table (tableKey(table))}
        <div
          class="table-row"
          class:active={selectedSchema === table.schema && selectedTable === table.name}
        >
          {#if editingKey === tableKey(table)}
            <input
              bind:this={editInputEl}
              class="table-item-edit-input"
              type="text"
              value={editValue}
              maxlength={MAX_DISPLAY_NAME_LENGTH}
              oninput={(event) => (editValue = (event.currentTarget as HTMLInputElement).value)}
              onclick={(event) => event.stopPropagation()}
              onkeydown={(event) => handleEditKeydown(event, table)}
              onblur={cancelEdit}
              aria-label={`Rename ${prettyLabel(table)}`}
              title={`${table.schema}.${table.name}`}
              data-testid={`table-item-rename-input-${tableKey(table)}`}
            />
          {:else}
            <button
              type="button"
              class="table-item"
              onclick={() => onSelect(table)}
              ondblclick={() => startEdit(table)}
              title={`${table.schema}.${table.name}`}
              data-testid={`table-item-${tableKey(table)}`}
            >
              <span class="table-item-name">{prettyLabel(table)}</span>
              {#if table.row_count_estimate != null}
                <span class="table-item-count">{table.row_count_estimate.toLocaleString()}</span>
              {/if}
            </button>
            <button
              type="button"
              class="table-item-rename-trigger"
              aria-label={`Rename ${prettyLabel(table)}`}
              onclick={(event) => {
                event.stopPropagation();
                startEdit(table);
              }}
              data-testid={`table-item-rename-trigger-${tableKey(table)}`}
            >
              <Pencil size={12} />
            </button>
          {/if}
        </div>
      {/each}
    {:else}
      <div class="empty-state" data-testid="table-list-empty">
        No tables match “{search.trim()}”.
      </div>
    {/if}
  </div>
</nav>

<style>
  .table-list {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-xs) 0;
  }

  .table-list.headerless {
    padding-top: 0;
  }

  .section-header {
    padding: 0 var(--sk-space-md);
  }

  .section-title {
    font-size: var(--sk-font-size-sm);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--sk-secondary-strong);
  }

  .section-subtitle {
    margin-top: var(--sk-space-xs);
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
  }

  /* search bar — padded to align with rows */
  .panel-search {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: 0 var(--sk-space-xs);
    color: var(--sk-muted);
  }

  .panel-search-input {
    width: 100%;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    padding: var(--sk-space-sm) var(--sk-space-sm);
    outline: none;
  }

  .panel-search-input:focus {
    border-color: rgba(var(--marble-active-rgb), 0.4);
    box-shadow: 0 0 0 2px var(--sk-ring);
  }

  .panel-search-input::placeholder {
    color: var(--sk-muted);
  }

  .list-items {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
  }

  /* row wrapper: hosts the select button + the hover-revealed rename trigger */
  .table-row {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--sk-radius-md);
  }

  /* roomier rows: 7px vertical padding, radius-md, marble-keyed hover */
  .table-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    flex: 1;
    min-width: 0;
    padding: 7px var(--sk-space-md);
    border: none;
    background: none;
    border-radius: var(--sk-radius-md);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
  }

  /* keyboard nav: consistent focus ring matching re-skin palette */
  .table-item:focus-visible {
    outline: 2px solid var(--sk-focus-ring);
    outline-offset: 2px;
  }

  /* hover: teal soft wash */
  .table-item:hover {
    background: var(--sk-active-tint-soft);
  }

  /* active: amber inset bar + ink-strong text */
  .table-row.active .table-item {
    background: var(--sk-active-tint-soft);
    color: var(--sk-ink-strong);
    font-weight: 500;
    box-shadow: inset 2px 0 0 var(--sk-accent-count);
  }

  .table-item-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* count: muted by default */
  .table-item-count {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  /* count on active row: amber chip */
  .table-row.active .table-item-count {
    color: var(--sk-accent-count-ink);
    background: var(--sk-count-chip-bg);
    border-radius: var(--sk-radius-sm);
    padding: 1px 6px;
  }

  /* pencil: hidden until the row is hovered/focused, no border/pill — flat icon-only */
  .table-item-rename-trigger {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    margin-right: var(--sk-space-xs);
    border: none;
    background: none;
    border-radius: var(--sk-radius-sm);
    color: var(--sk-muted);
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .table-row:hover .table-item-rename-trigger,
  .table-row:focus-within .table-item-rename-trigger {
    opacity: 1;
  }

  .table-item-rename-trigger:hover {
    background: var(--sk-active-tint-soft);
    color: var(--sk-text);
  }

  .table-item-rename-trigger:focus-visible {
    opacity: 1;
    outline: 2px solid var(--sk-focus-ring);
    outline-offset: 2px;
  }

  /* inline rename input: same footprint as .table-item, no border/pill chip */
  .table-item-edit-input {
    flex: 1;
    min-width: 0;
    padding: 7px var(--sk-space-md);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    outline: none;
  }

  .table-item-edit-input:focus {
    border-color: rgba(var(--marble-active-rgb), 0.4);
    box-shadow: 0 0 0 2px var(--sk-ring);
  }

  .empty-state {
    padding: var(--sk-space-sm) var(--sk-space-md);
    color: var(--sk-muted);
    font-size: var(--sk-font-size-body);
  }
</style>
