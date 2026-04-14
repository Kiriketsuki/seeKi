<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TableList from './components/TableList.svelte';
  import ActionDock from './components/ActionDock.svelte';
  import TableHeader from './components/TableHeader.svelte';
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
  import { COLUMN_VISIBILITY_KEY_PREFIX, SIDEBAR_COLLAPSED_KEY } from './lib/constants';
  import SetupWizard from './components/SetupWizard.svelte';

  let tables: TableInfo[] = $state([]);
  let selectedSchema: string = $state('');
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
  let searchTerm: string = $state('');
  let searchVisible: boolean = $state(false);
  let columnsOpen: boolean = $state(false);
  let columnVisibility: Record<string, boolean> = $state({});
  let searchInputEl: HTMLInputElement | null = $state(null);
  let filterDebounceId: ReturnType<typeof setTimeout> | null = null;
  let searchDebounceId: ReturnType<typeof setTimeout> | null = null;
  let selectRequestId = 0;
  let activeFilterCount = $derived(
    Object.values(filters).filter((value) => value.trim().length > 0).length
  );
  let searchQuery = $derived.by(() => searchTerm.trim());
  let searchActive = $derived.by(() => searchVisible || searchQuery.length > 0);
  let hiddenColumnCount = $derived.by(
    () => columns.filter((column) => columnVisibility[column.name] === false).length
  );
  let visibleColumns = $derived.by(
    () => columns.filter((column) => columnVisibility[column.name] !== false)
  );
  let selectedTableKey = $derived.by(() =>
    selectedSchema && selectedTable ? `${selectedSchema}.${selectedTable}` : ''
  );
  let selectedTableDisplayName = $derived.by(
    () => displayConfig?.tables[selectedTableKey]?.display_name ?? selectedTable
  );

  onMount(async () => {
    try {
      const status = await fetchStatus();
      if (status.mode === 'setup') {
        isSetup = true;
        return;
      }
      const [fetchedTables, config] = await Promise.all([
        fetchTables(),
        fetchDisplayConfig(),
      ]);
      tables = fetchedTables;
      displayConfig = config;
      if (tables.length > 0) {
        await selectTable(tables[0]);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to connect to database';
    } finally {
      loading = false;
    }
  });

  onMount(() => {
    function isTextEditingTarget(target: EventTarget | null): boolean {
      if (!(target instanceof HTMLElement)) return false;
      const tag = target.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
      return target.isContentEditable;
    }

    function handleKeydown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.altKey) return;

      const key = event.key.toLowerCase();
      const isShortcut = event.ctrlKey || event.metaKey;
      const inTextField = isTextEditingTarget(event.target);

      const isSearchField = event.target === searchInputEl;
      if (isShortcut && key === 'k' && (!inTextField || isSearchField)) {
        event.preventDefault();
        toggleSearch();
        return;
      }

      if (isShortcut && key === 'f' && !inTextField) {
        event.preventDefault();
        toggleFilters();
        return;
      }

      if (event.key === 'Escape') {
        if (columnsOpen) {
          event.preventDefault();
          columnsOpen = false;
          return;
        }

        if (searchVisible || searchQuery.length > 0) {
          event.preventDefault();
          handleSearchClear();
        }
      }
    }

    window.addEventListener('keydown', handleKeydown);
    return () => {
      window.removeEventListener('keydown', handleKeydown);
      clearFilterDebounce();
      clearSearchDebounce();
    };
  });

  function clearFilterDebounce() {
    if (filterDebounceId != null) {
      clearTimeout(filterDebounceId);
      filterDebounceId = null;
    }
  }

  function clearSearchDebounce() {
    if (searchDebounceId != null) {
      clearTimeout(searchDebounceId);
      searchDebounceId = null;
    }
  }

  function normalizeColumnVisibility(
    tableColumns: ColumnInfo[],
    visibility: Record<string, boolean>,
  ): Record<string, boolean> {
    return Object.fromEntries(
      tableColumns.map((column) => [column.name, visibility[column.name] !== false])
    );
  }

  function loadColumnVisibility(
    tableName: string,
    tableColumns: ColumnInfo[],
  ): Record<string, boolean> {
    if (typeof localStorage === 'undefined') {
      return normalizeColumnVisibility(tableColumns, {});
    }

    // Key format changed in PR #57: tableName is now "schema.table" (e.g. "reporting.orders"),
    // not the bare table name used before. A future migration would need to read old bare-name
    // keys and translate them — no migration code here.
    const storageKey = `${COLUMN_VISIBILITY_KEY_PREFIX}${tableName}`;
    const raw = localStorage.getItem(storageKey);
    if (!raw) {
      return normalizeColumnVisibility(tableColumns, {});
    }

    try {
      const parsed = JSON.parse(raw) as Record<string, unknown>;
      const nextVisibility: Record<string, boolean> = {};
      for (const column of tableColumns) {
        nextVisibility[column.name] = parsed[column.name] === false ? false : true;
      }
      return nextVisibility;
    } catch {
      return normalizeColumnVisibility(tableColumns, {});
    }
  }

  function persistColumnVisibility(
    tableName: string,
    tableColumns: ColumnInfo[],
    visibility: Record<string, boolean>,
  ) {
    if (typeof localStorage === 'undefined') {
      return;
    }

    const storageKey = `${COLUMN_VISIBILITY_KEY_PREFIX}${tableName}`;
    try {
      localStorage.setItem(
        storageKey,
        JSON.stringify(normalizeColumnVisibility(tableColumns, visibility))
      );
    } catch {
      // Degrade to in-memory only (e.g. Safari private mode QuotaExceededError)
    }
  }

  function resetSearchState() {
    clearSearchDebounce();
    searchTerm = '';
    searchVisible = false;
  }

  async function openSearch() {
    searchVisible = true;
    await tick();
    searchInputEl?.focus();
    searchInputEl?.select();
  }

  function toggleSearch() {
    if (searchVisible) {
      searchVisible = false;
      return;
    }

    columnsOpen = false;
    void openSearch();
  }

  function toggleFilters() {
    filtersVisible = !filtersVisible;
  }

  function toggleColumns() {
    columnsOpen = !columnsOpen;
    if (columnsOpen) {
      searchVisible = false;
    }
  }

  function closeColumns() {
    columnsOpen = false;
  }

  function setSearchInputEl(node: HTMLInputElement | null) {
    searchInputEl = node;
  }

  async function selectTable(table: TableInfo) {
    const myRequest = ++selectRequestId;
    const resetSortState: SortState = { column: null, direction: null };
    const resetFilters: FilterState = {};
    selectedSchema = table.schema;
    selectedTable = table.name;
    tableError = null;
    tableLoading = true;
    currentPage = 1;
    sortState = resetSortState;
    filtersVisible = false;
    filters = resetFilters;
    columnsOpen = false;
    clearFilterDebounce();
    resetSearchState();
    const storageKey = `${table.schema}.${table.name}`;
    try {
      const [cols, result] = await Promise.all([
        fetchColumns(table.schema, table.name),
        fetchRows(
          table.schema,
          table.name,
          buildRowsParams(1, resetSortState, resetFilters, ''),
        ),
      ]);
      if (myRequest !== selectRequestId) return;
      columns = cols;
      columnVisibility = loadColumnVisibility(storageKey, cols);
      queryResult = result;
    } catch (e) {
      if (myRequest !== selectRequestId) return;
      tableError = e instanceof Error ? e.message : 'Failed to load table';
    } finally {
      if (myRequest === selectRequestId) tableLoading = false;
    }
  }

  function buildRowsParams(
    page: number,
    nextSortState: SortState = sortState,
    nextFilters: FilterState = filters,
    nextSearchTerm: string = searchTerm,
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

    const trimmedSearch = nextSearchTerm.trim();
    if (trimmedSearch.length > 0) {
      params.search = trimmedSearch;
    }

    return params;
  }

  async function loadRows(
    page: number,
    nextSortState: SortState = sortState,
    nextFilters: FilterState = filters,
    nextSearchTerm: string = searchTerm,
  ) {
    if (!selectedTable || !selectedSchema) return;
    const myRequest = ++selectRequestId;
    tableError = null;
    tableLoading = true;
    try {
      const result = await fetchRows(
        selectedSchema,
        selectedTable,
        buildRowsParams(page, nextSortState, nextFilters, nextSearchTerm)
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
    clearSearchDebounce();
    await loadRows(page);
  }

  function handleSortChange(column: string, direction: SortDirection | null) {
    clearFilterDebounce();
    clearSearchDebounce();
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
    clearSearchDebounce();
    filterDebounceId = setTimeout(() => {
      void loadRows(1, sortState, nextFilters);
      filterDebounceId = null;
    }, 300);
  }

  function scheduleSearchReload() {
    clearSearchDebounce();
    clearFilterDebounce();
    if (!selectedTable) return;

    searchDebounceId = setTimeout(() => {
      void loadRows(1);
      searchDebounceId = null;
    }, 300);
  }

  function handleSearchInput(event: Event) {
    searchTerm = (event.currentTarget as HTMLInputElement).value;
    scheduleSearchReload();
  }

  function handleSearchClear() {
    clearFilterDebounce();
    resetSearchState();
    if (!selectedTable) {
      return;
    }

    void loadRows(1, sortState, filters, '');
  }

  function handleToggleColumnVisibility(columnName: string, visible: boolean) {
    if (!selectedTable || !selectedTableKey) return;

    const nextVisibility = normalizeColumnVisibility(columns, {
      ...columnVisibility,
      [columnName]: visible,
    });
    columnVisibility = nextVisibility;
    persistColumnVisibility(selectedTableKey, columns, nextVisibility);
  }

  function handleShowAllColumns() {
    if (!selectedTable || !selectedTableKey) return;

    const nextVisibility = normalizeColumnVisibility(
      columns,
      Object.fromEntries(columns.map((column) => [column.name, true])) as Record<string, boolean>
    );
    columnVisibility = nextVisibility;
    persistColumnVisibility(selectedTableKey, columns, nextVisibility);
  }

  function exportCsv() {
    if (!selectedTable || !selectedSchema) return;

    const params = buildRowsParams(1);
    const searchParams = new URLSearchParams();
    if (params.sort_column) searchParams.set('sort_column', params.sort_column);
    if (params.sort_direction) searchParams.set('sort_direction', params.sort_direction);
    if (params.search) searchParams.set('search', params.search);
    if (params.filters) {
      for (const [col, val] of Object.entries(params.filters)) {
        searchParams.set(`filter.${col}`, val);
      }
    }
    const qs = searchParams.toString();
    const base = `/api/export/${encodeURIComponent(selectedSchema)}/${encodeURIComponent(selectedTable)}/csv`;
    window.open(`${base}${qs ? `?${qs}` : ''}`, '_blank');
  }
</script>

{#if isSetup}
  <SetupWizard />
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
        <TableList {tables} {selectedSchema} {selectedTable} onSelect={selectTable} />
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
        <TableList {tables} {selectedSchema} {selectedTable} onSelect={selectTable} />
      {/if}
    </Sidebar>
    <main class="main">
      <div class="table-panel">
        <TableHeader tableName={selectedTableDisplayName} rowCount={queryResult?.total_rows ?? 0} />
      </div>
      {#if tableError}
        <div class="table-error-banner">
          <span>{tableError}</span>
          <button class="dismiss-btn" onclick={() => tableError = null}>Dismiss</button>
        </div>
      {/if}
      <div class="grid-area">
        <div class="grid-shell">
          <div class="grid-content">
            <DataGrid
              columns={visibleColumns}
              rows={queryResult?.rows ?? []}
              {sortState}
              {filters}
              {filtersVisible}
              onSortChange={handleSortChange}
              onFilterChange={handleFilterChange}
            />
          </div>
          {#if selectedTable}
            <ActionDock
              searchVisible={searchVisible}
              searchTerm={searchTerm}
              searchActive={searchActive}
              filtersVisible={filtersVisible}
              activeFilterCount={activeFilterCount}
              columnsOpen={columnsOpen}
              columns={columns}
              columnVisibility={columnVisibility}
              hiddenColumnCount={hiddenColumnCount}
              hasTable={!!selectedTable}
              disabled={tableLoading}
              onToggleSearch={toggleSearch}
              onSearchInput={handleSearchInput}
              onSearchClear={handleSearchClear}
              onToggleFilters={toggleFilters}
              onToggleColumns={toggleColumns}
              onToggleColumnVisibility={handleToggleColumnVisibility}
              onShowAllColumns={handleShowAllColumns}
              onCloseColumns={closeColumns}
              onExport={exportCsv}
              onSearchInputRef={setSearchInputEl}
            />
          {/if}
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

  .table-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-lg) var(--sk-space-2xl) 0;
  }

  .grid-area {
    flex: 1;
    min-height: 0;
    display: flex;
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

  .grid-content {
    position: absolute;
    inset: 0 0 var(--sk-dock-clearance) 0;
    min-width: 0;
    min-height: 0;
  }

  .grid-loading {
    position: absolute;
    inset: 0 0 var(--sk-dock-clearance) 0;
    z-index: 2;
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
    to {
      transform: rotate(360deg);
    }
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
