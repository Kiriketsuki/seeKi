<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TableList from './components/TableList.svelte';
  import Toolbar from './components/Toolbar.svelte';
  import ToolStrip from './components/ToolStrip.svelte';
  import DataGrid from './components/DataGrid.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import { fetchTables, fetchColumns, fetchRows, fetchDisplayConfig, fetchStatus } from './lib/api';
  import type { FetchRowsParams } from './lib/api';
  import type {
    TableInfo,
    ColumnInfo,
    QueryResult,
    DisplayConfig,
    SortState,
    FilterState,
    SortDirection,
  } from './lib/types';
  import { SIDEBAR_COLLAPSED_KEY } from './lib/constants';

  let tables: TableInfo[] = $state([]);
  let selectedTable: string = $state('');
  let columns: ColumnInfo[] = $state([]);
  let queryResult: QueryResult | null = $state(null);
  let displayConfig: DisplayConfig | null = $state(null);
  let sidebarCollapsed: boolean = $state(
    typeof localStorage !== 'undefined' && localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === 'true'
  );
  let isSetup: boolean = $state(false);
  let loading: boolean = $state(true);
  let tableLoading: boolean = $state(false);
  let error: string | null = $state(null);
  let tableError: string | null = $state(null);
  let currentPage: number = $state(1);
  let sortState: SortState = $state({ column: null, direction: null });
  let filtersVisible: boolean = $state(false);
  let filters: FilterState = $state({});
  let filterDebounceId: ReturnType<typeof setTimeout> | null = null;
  let selectRequestId = 0;
  let activeFilterCount = $derived(
    Object.values(filters).filter((value) => value.trim().length > 0).length
  );
  let sortLabel = $derived.by(() => {
    if (!sortState.column || !sortState.direction) {
      return 'No active sort';
    }

    const currentColumn = columns.find((column) => column.name === sortState.column);
    const displayName = currentColumn?.display_name ?? sortState.column;
    return `${displayName} ${sortState.direction}`;
  });

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
    } finally {
      loading = false;
    }
  });

  onMount(() => {
    function handleKeydown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.altKey) return;
      if (!event.ctrlKey && !event.metaKey) return;
      if (event.key.toLowerCase() !== 'f') return;

      event.preventDefault();
      toggleFilters();
    }

    window.addEventListener('keydown', handleKeydown);
    return () => {
      window.removeEventListener('keydown', handleKeydown);
      clearFilterDebounce();
    };
  });

  async function selectTable(tableName: string) {
    const myRequest = ++selectRequestId;
    const resetSortState: SortState = { column: null, direction: null };
    const resetFilters: FilterState = {};
    selectedTable = tableName;
    tableError = null;
    tableLoading = true;
    currentPage = 1;
    sortState = resetSortState;
    filtersVisible = false;
    filters = resetFilters;
    clearFilterDebounce();
    try {
      const [cols, result] = await Promise.all([
        fetchColumns(tableName),
        fetchRows(tableName, buildRowsParams(1, resetSortState, resetFilters))
      ]);
      if (myRequest !== selectRequestId) return;
      columns = cols;
      queryResult = result;
    } catch (e) {
      if (myRequest !== selectRequestId) return;
      tableError = e instanceof Error ? e.message : 'Failed to load table';
    } finally {
      if (myRequest === selectRequestId) tableLoading = false;
    }
  }

  function toggleFilters() {
    filtersVisible = !filtersVisible;
  }

  function clearFilterDebounce() {
    if (filterDebounceId != null) {
      clearTimeout(filterDebounceId);
      filterDebounceId = null;
    }
  }

  function buildRowsParams(
    page: number,
    nextSortState: SortState = sortState,
    nextFilters: FilterState = filters,
  ): FetchRowsParams {
    const params: FetchRowsParams = { page };
    if (nextSortState.column && nextSortState.direction) {
      params.sort_column = nextSortState.column;
      params.sort_direction = nextSortState.direction;
    }

    const activeFilters = Object.fromEntries(
      Object.entries(nextFilters).filter(([, value]) => value.trim().length > 0)
    );
    if (Object.keys(activeFilters).length > 0) {
      params.filters = activeFilters;
    }

    return params;
  }

  async function loadRows(
    page: number,
    nextSortState: SortState = sortState,
    nextFilters: FilterState = filters,
  ) {
    if (!selectedTable) return;
    const myRequest = ++selectRequestId;
    tableError = null;
    tableLoading = true;
    try {
      const result = await fetchRows(
        selectedTable,
        buildRowsParams(page, nextSortState, nextFilters)
      );
      if (myRequest !== selectRequestId) return;
      queryResult = result;
      currentPage = page;
    } catch (e) {
      if (myRequest !== selectRequestId) return;
      tableError = e instanceof Error ? e.message : 'Failed to load rows';
    } finally {
      if (myRequest === selectRequestId) tableLoading = false;
    }
  }

  async function goToPage(page: number) {
    clearFilterDebounce();
    await loadRows(page);
  }

  function handleSortChange(column: string, direction: SortDirection | null) {
    clearFilterDebounce();
    const nextSortState: SortState = direction
      ? { column, direction }
      : { column: null, direction: null };
    sortState = nextSortState;
    void loadRows(1, nextSortState);
  }

  function handleFilterChange(column: string, value: string) {
    const nextFilters: FilterState = {
      ...filters,
      [column]: value,
    };
    filters = nextFilters;

    clearFilterDebounce();
    filterDebounceId = setTimeout(() => {
      void loadRows(1, sortState, nextFilters);
      filterDebounceId = null;
    }, 300);
  }

  function exportCsv() {
    if (!selectedTable) return;
    const params = buildRowsParams(1);
    const searchParams = new URLSearchParams();
    if (params.sort_column) searchParams.set('sort_column', params.sort_column);
    if (params.sort_direction) searchParams.set('sort_direction', params.sort_direction);
    if (params.filters) {
      for (const [col, val] of Object.entries(params.filters)) {
        searchParams.set(`filter.${col}`, val);
      }
    }
    const qs = searchParams.toString();
    window.open(`/api/export/${encodeURIComponent(selectedTable)}/csv${qs ? `?${qs}` : ''}`, '_blank');
  }
</script>

{#if isSetup}
  <!-- Setup wizard placeholder — Epic 5 -->
  <div>Setup wizard will go here</div>
{:else if loading}
  <div class="layout">
    <div class="loading-state">
      <div class="loading-spinner"></div>
      <p>Connecting to database...</p>
    </div>
  </div>
{:else if error}
  <div class="layout">
    <Sidebar
      bind:collapsed={sidebarCollapsed}
      onToggle={() => sidebarCollapsed = !sidebarCollapsed}
      title="SeeKi"
      subtitle=""
    >
      {#if !sidebarCollapsed}
        <TableList {tables} {selectedTable} onSelect={selectTable} />
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
        <TableList {tables} {selectedTable} onSelect={selectTable} />
      {/if}
    </Sidebar>
    <main class="main">
      <Toolbar
        tableName={displayConfig?.tables[selectedTable]?.display_name ?? selectedTable}
        rowCount={queryResult?.total_rows ?? 0}
        onExport={exportCsv}
      />
      {#if tableError}
        <div class="table-error-banner">
          <span>{tableError}</span>
          <button class="dismiss-btn" onclick={() => tableError = null}>Dismiss</button>
        </div>
      {/if}
      <div class="grid-area">
        <ToolStrip
          {sortState}
          sortDescription={sortLabel}
          filtersVisible={filtersVisible}
          activeFilterCount={activeFilterCount}
          onToggleFilters={toggleFilters}
        />
        <div class="grid-shell" class:loading-overlay={tableLoading}>
          <DataGrid
            {columns}
            rows={queryResult?.rows ?? []}
            {sortState}
            {filters}
            {filtersVisible}
            onSortChange={handleSortChange}
            onFilterChange={handleFilterChange}
          />
          {#if tableLoading}
            <div class="grid-loading">
              <div class="loading-spinner"></div>
            </div>
          {/if}
        </div>
      </div>
      <StatusBar
        total={queryResult?.total_rows ?? 0}
        start={queryResult && queryResult.total_rows > 0 ? (queryResult.page - 1) * queryResult.page_size + 1 : 0}
        end={queryResult && queryResult.total_rows > 0 ? Math.min(queryResult.page * queryResult.page_size, queryResult.total_rows) : 0}
        page={queryResult?.page ?? 1}
        totalPages={queryResult ? Math.max(1, Math.ceil(queryResult.total_rows / queryResult.page_size)) : 1}
        loading={tableLoading}
        onPageChange={goToPage}
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
    min-height: 0;
  }
  .grid-area {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: var(--sk-space-md);
    padding: var(--sk-space-lg) var(--sk-space-2xl);
    overflow: hidden;
    align-items: stretch;
  }
  .grid-shell {
    flex: 1;
    min-width: 0;
    min-height: 0;
    position: relative;
  }
  .loading-overlay {
    opacity: 0.5;
    pointer-events: none;
  }
  .grid-loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(245, 240, 235, 0.28);
  }
  .loading-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-md);
    color: var(--sk-muted);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
  }
  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--sk-border);
    border-top-color: var(--sk-accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
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
