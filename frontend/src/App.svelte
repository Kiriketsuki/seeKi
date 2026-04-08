<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Toolbar from './components/Toolbar.svelte';
  import DataGrid from './components/DataGrid.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import { fetchTables, fetchColumns, fetchRows, fetchDisplayConfig } from './lib/api';
  import type { TableInfo, ColumnInfo, QueryResult, DisplayConfig } from './lib/types';

  let tables: TableInfo[] = $state([]);
  let selectedTable: string = $state('');
  let columns: ColumnInfo[] = $state([]);
  let queryResult: QueryResult | null = $state(null);
  let displayConfig: DisplayConfig | null = $state(null);
  let sidebarCollapsed: boolean = $state(false);
  let isSetup = false; // future: detect setup mode from API

  onMount(async () => {
    const [fetchedTables, config] = await Promise.all([
      fetchTables(),
      fetchDisplayConfig()
    ]);
    tables = fetchedTables;
    displayConfig = config;
    if (tables.length > 0) {
      await selectTable(tables[0].name);
    }
  });

  async function selectTable(tableName: string) {
    selectedTable = tableName;
    const [cols, result] = await Promise.all([
      fetchColumns(tableName),
      fetchRows(tableName)
    ]);
    columns = cols;
    queryResult = result;
  }
</script>

{#if isSetup}
  <!-- Setup wizard placeholder — Epic 5 -->
  <div>Setup wizard will go here</div>
{:else}
  <div class="layout">
    <Sidebar
      bind:collapsed={sidebarCollapsed}
      onToggle={() => sidebarCollapsed = !sidebarCollapsed}
      title={displayConfig?.branding?.title ?? 'SeeKi'}
      subtitle={displayConfig?.branding?.subtitle ?? ''}
    />
    <main class="main">
      <Toolbar
        tableName={selectedTable}
        rowCount={queryResult?.total_rows ?? 0}
      />
      <div class="grid-area">
        <DataGrid {columns} rows={queryResult?.rows ?? []} />
      </div>
      <StatusBar
        total={queryResult?.total_rows ?? 0}
        start={queryResult ? (queryResult.page - 1) * queryResult.page_size + 1 : 0}
        end={queryResult ? Math.min(queryResult.page * queryResult.page_size, queryResult.total_rows) : 0}
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
</style>
