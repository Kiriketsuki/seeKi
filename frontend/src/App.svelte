<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Toolbar from './components/Toolbar.svelte';
  import DataGrid from './components/DataGrid.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import { fetchTables, fetchColumns, fetchRows, fetchDisplayConfig, fetchStatus } from './lib/api';
  import type { TableInfo, ColumnInfo, QueryResult, DisplayConfig } from './lib/types';

  let tables: TableInfo[] = $state([]);
  let selectedTable: string = $state('');
  let columns: ColumnInfo[] = $state([]);
  let queryResult: QueryResult | null = $state(null);
  let displayConfig: DisplayConfig | null = $state(null);
  let sidebarCollapsed: boolean = $state(false);
  let isSetup: boolean = $state(false);
  let error: string | null = $state(null);
  let tableError: string | null = $state(null);

  onMount(async () => {
    try {
      const status = await fetchStatus();
      if (status.mode === 'setup') {
        isSetup = true;
        return;
      }
      const [fetchedTables, config] = await Promise.all([
        fetchTables(),
        fetchDisplayConfig()
      ]);
      tables = fetchedTables;
      displayConfig = config;
      if (tables.length > 0) {
        await selectTable(tables[0].name);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to connect to database';
    }
  });

  async function selectTable(tableName: string) {
    tableError = null;
    try {
      const [cols, result] = await Promise.all([
        fetchColumns(tableName),
        fetchRows(tableName)
      ]);
      selectedTable = tableName;
      columns = cols;
      queryResult = result;
    } catch (e) {
      tableError = e instanceof Error ? e.message : 'Failed to load table';
    }
  }
</script>

{#if isSetup}
  <!-- Setup wizard placeholder — Epic 5 -->
  <div>Setup wizard will go here</div>
{:else if error}
  <div class="layout">
    <Sidebar
      bind:collapsed={sidebarCollapsed}
      onToggle={() => sidebarCollapsed = !sidebarCollapsed}
      title="SeeKi"
      subtitle=""
    >
      {#if !sidebarCollapsed}
        <nav class="table-list">
          {#each tables as table}
            <button
              class="table-item"
              class:active={selectedTable === table.name}
              onclick={() => selectTable(table.name)}
            >
              <span class="table-item-name">{table.display_name}</span>
              {#if table.row_count_estimate != null}
                <span class="table-item-count">{table.row_count_estimate.toLocaleString()}</span>
              {/if}
            </button>
          {/each}
        </nav>
      {/if}
    </Sidebar>
    <main class="main">
      <div class="error-state">
        <div class="error-card">
          <h2>Could not reach database</h2>
          <p>{error}</p>
          <button class="retry-btn" onclick={() => location.reload()}>Retry</button>
        </div>
      </div>
    </main>
  </div>
{:else}
  <div class="layout">
    <Sidebar
      bind:collapsed={sidebarCollapsed}
      onToggle={() => sidebarCollapsed = !sidebarCollapsed}
      title={displayConfig?.branding?.title ?? 'SeeKi'}
      subtitle={displayConfig?.branding?.subtitle ?? ''}
    >
      {#if !sidebarCollapsed}
        <nav class="table-list">
          {#each tables as table}
            <button
              class="table-item"
              class:active={selectedTable === table.name}
              onclick={() => selectTable(table.name)}
            >
              <span class="table-item-name">{table.display_name}</span>
              {#if table.row_count_estimate != null}
                <span class="table-item-count">{table.row_count_estimate.toLocaleString()}</span>
              {/if}
            </button>
          {/each}
        </nav>
      {/if}
    </Sidebar>
    <main class="main">
      <Toolbar
        tableName={selectedTable}
        rowCount={queryResult?.total_rows ?? 0}
      />
      {#if tableError}
        <div class="table-error-banner">
          <span>{tableError}</span>
          <button class="dismiss-btn" onclick={() => tableError = null}>Dismiss</button>
        </div>
      {/if}
      <div class="grid-area">
        <DataGrid {columns} rows={queryResult?.rows ?? []} />
      </div>
      <StatusBar
        total={queryResult?.total_rows ?? 0}
        start={queryResult && queryResult.total_rows > 0 ? (queryResult.page - 1) * queryResult.page_size + 1 : 0}
        end={queryResult && queryResult.total_rows > 0 ? Math.min(queryResult.page * queryResult.page_size, queryResult.total_rows) : 0}
        page={queryResult?.page ?? 1}
        totalPages={queryResult ? Math.ceil(queryResult.total_rows / queryResult.page_size) : 1}
      />
    </main>
  </div>
{/if}

<style>
  .layout {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .grid-area {
    flex: 1;
    padding: var(--sk-space-lg) var(--sk-space-2xl);
    overflow: auto;
  }
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
  .table-error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-md);
    padding: var(--sk-space-sm) var(--sk-space-2xl);
    background: rgba(220, 38, 38, 0.1);
    border-bottom: 1px solid rgba(220, 38, 38, 0.3);
    color: var(--sk-text);
    font-size: var(--sk-font-size-body);
  }
  .dismiss-btn {
    background: none;
    border: 1px solid rgba(220, 38, 38, 0.4);
    border-radius: var(--sk-radius-sm);
    padding: 2px var(--sk-space-sm);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-sm);
    color: var(--sk-text);
    cursor: pointer;
  }
  .error-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sk-space-2xl);
  }
  .error-card {
    background: var(--sk-glass-grid);
    backdrop-filter: var(--sk-glass-grid-blur);
    -webkit-backdrop-filter: var(--sk-glass-grid-blur);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    box-shadow: var(--sk-shadow-card);
    padding: var(--sk-space-2xl) 40px;
    text-align: center;
    max-width: 420px;
  }
  .error-card h2 {
    font-size: var(--sk-font-size-xl);
    color: var(--sk-text);
    margin-bottom: var(--sk-space-sm);
  }
  .error-card p {
    font-size: var(--sk-font-size-body);
    color: var(--sk-muted);
    margin-bottom: var(--sk-space-lg);
  }
  .retry-btn {
    background: var(--sk-accent);
    color: white;
    border: none;
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-xs) var(--sk-space-lg);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    cursor: pointer;
  }
</style>
