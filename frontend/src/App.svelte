<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import SettingsNav from './components/SettingsNav.svelte';
  import SettingsContent from './components/SettingsContent.svelte';
  import TableList from './components/TableList.svelte';
  import DataPanels from './components/DataPanels.svelte';
  import ViewBuilder from './components/ViewBuilder.svelte';
  import ActionDock from './components/ActionDock.svelte';
  import TableHeader from './components/TableHeader.svelte';
  import DataGrid from './components/DataGrid.svelte';
  import GridRefreshToolbar from './components/GridRefreshToolbar.svelte';
  import QuickStatsBar from './components/QuickStatsBar.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import RowCapWarning from './components/RowCapWarning.svelte';
  import SetupWizard from './components/SetupWizard.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import {
    buildViewCsvUrl,
    deleteView,
    fetchColumns,
    fetchDisplayConfig,
    fetchLastUsedState,
    fetchRows,
    fetchSettings,
    fetchStatus,
    fetchTables,
    fetchUpdateStatus,
    fetchView,
    fetchViewRows,
    fetchViews,
    renameView,
    saveLastUsedState,
    saveSettings,
  } from './lib/api';
  import type { FetchRowsParams } from './lib/api';
  import type {
    AppearanceSettings,
    BrandingSettings,
    ColumnInfo,
    DisplayConfig,
    FilterState,
    PageSizePreference,
    PaginationMode,
    QueryResult,
    SavedViewDefinition,
    SavedViewSummary,
    SettingsEntries,
    SidebarMode,
    SortColumn,
    SortState,
    TableInfo,
    TablesSurface,
    UpdateStatus,
    ViewColumn,
    ViewDraft,
    ViewDefinitionFilters,
    ViewGrouping,
    ViewRanking,
    ViewSourceRef,
  } from './lib/types';
  import { COLUMN_VISIBILITY_KEY_PREFIX, SIDEBAR_COLLAPSED_KEY } from './lib/constants';
  import {
    createGridRefreshController,
    sidebarMode,
    type GridRefreshSnapshot,
  } from './lib/stores';
  import {
    buildAppearanceSettingsEntries,
    buildBrandingSettingsEntries,
    buildDataSettingsEntries,
    isPageSizePreference,
    parseAppearanceSettings,
    parseBrandingSettings,
    parseDataSettings,
  } from './lib/settings';
  import {
    appendBatch,
    computeHasMore,
    isSyntheticRow,
    makeInlineErrorRow,
    makeSyntheticSkeletonRows,
    resetState,
    type RowCapState,
  } from './lib/infinite-scroll';

  function setSidebarMode(mode: SidebarMode) {
    sidebarMode.set(mode);
  }

  function normalizeViewFilters(
    source: Record<string, unknown> | undefined,
  ): ViewDefinitionFilters {
    const entries = Object.entries(source ?? {}).map(([key, value]) => {
      if (typeof value === 'string') {
        return [key, { op: 'contains', value }] as const;
      }
      return [key, structuredClone(value)] as const;
    });
    return Object.fromEntries(entries) as ViewDefinitionFilters;
  }

  function cloneDraft(source: ViewDraft): ViewDraft {
    return {
      name: source.name,
      base_schema: source.base_schema,
      base_table: source.base_table,
      definition_version: source.definition_version,
      columns: source.columns.map((column) => ({ ...column })),
      filters: normalizeViewFilters(source.filters),
      sources: source.sources?.map((sourceRef) => structuredClone(sourceRef)) ?? [],
      grouping: source.grouping ? structuredClone(source.grouping) : null,
      ranking: source.ranking ? structuredClone(source.ranking) : null,
      template: source.template ?? null,
    };
  }

  function draftFromView(view: SavedViewDefinition, name = `${view.name} copy`): ViewDraft {
    return {
      name,
      base_schema: view.base_schema,
      base_table: view.base_table,
      definition_version: view.definition_version,
      columns: view.columns.map((column) => ({ ...column })),
      filters: normalizeViewFilters(view.filters),
      sources: view.sources?.map((sourceRef) => structuredClone(sourceRef)) ?? [],
      grouping: view.grouping ? structuredClone(view.grouping) : null,
      ranking: view.ranking ? structuredClone(view.ranking) : null,
      template: view.template ?? null,
    };
  }

  let tables: TableInfo[] = $state([]);
  let savedViews: SavedViewSummary[] = $state([]);
  let tablesSurface: TablesSurface = $state({ kind: 'table' });
  let builderDraft: ViewDraft | null = $state(null);
  let builderDraftLive: { columns: ViewColumn[]; sources: ViewSourceRef[]; grouping: ViewGrouping | null; ranking: ViewRanking | null } | null = $state(null);
  let builderSourceLabel = $state('');
  let builderReturnTarget: TablesSurface = $state({ kind: 'table' });
  let pendingCreateView = $state(false);
  let selectedSchema: string = $state('');
  let selectedTable: string = $state('');
  let selectedView: SavedViewDefinition | null = $state(null);
  let columns: ColumnInfo[] = $state([]);
  let queryResult: QueryResult | null = $state(null);
  let displayConfig: DisplayConfig | null = $state(null);
  let appSettings: SettingsEntries = $state({});
  let sidebarCollapsed: boolean = $state(
    typeof localStorage !== 'undefined' &&
      localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === 'true'
  );
  let isSetup: boolean = $state(false);
  let loading: boolean = $state(true);
  let tableLoading: boolean = $state(false);
  let error: string | null = $state(null);
  let tableError: string | null = $state(null);
  let currentPage: number = $state(1);
  let sortState: SortState = $state([]);
  let filtersVisible: boolean = $state(false);
  let filters: FilterState = $state({});
  let searchTerm: string = $state('');
  let searchVisible: boolean = $state(false);
  let columnsOpen: boolean = $state(false);
  let columnVisibility: Record<string, boolean> = $state({});
  let searchInputEl: HTMLInputElement | null = $state(null);
  let searchButtonEl: HTMLButtonElement | null = $state(null);
  let columnsButtonEl: HTMLButtonElement | null = $state(null);
  let filterButtonEl: HTMLButtonElement | null = $state(null);
  let settingsOpen: boolean = $state(false);
  let updateAvailable: boolean = $state(false);
  let updateStatus: UpdateStatus | null = $state(null);
  let pageSize: PageSizePreference = $state(50);
  let paginationMode: PaginationMode = $state('infinite');
  let loadedRows: Record<string, unknown>[] = $state([]);
  let lastLoadedPage: number = $state(0);
  let fetchingMore: boolean = $state(false);
  let appendError: boolean = $state(false);
  let rowCapState: RowCapState = $state('none');
  let resetSignal: number = $state(0);
  let filterDebounceId: ReturnType<typeof setTimeout> | null = null;
  let searchDebounceId: ReturnType<typeof setTimeout> | null = null;
  let lastUsedSaveId: ReturnType<typeof setTimeout> | null = null;
  let modeShortcutId: ReturnType<typeof setTimeout> | null = null;
  let updateStatusPollId: ReturnType<typeof setInterval> | null = null;
  let pendingModeShortcut: 'g' | null = null;
  let selectRequestId = 0;
  let refreshSnapshot: GridRefreshSnapshot = $state({
    surfaceKey: null,
    intervalMs: 0,
    lastRefreshedAt: null,
    inFlight: false,
  });

  const refreshController = createGridRefreshController(async () => {
    if (paginationMode === 'infinite') {
      await resetAndLoadRows(sortState, filters, searchTerm);
    } else {
      await loadRows(currentPage, sortState, filters, searchTerm);
    }
  });

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
  let focusedTextColumnName = $derived.by(() => {
    const preferred = visibleColumns.find((column) =>
      !['smallint', 'integer', 'bigint', 'real', 'double precision', 'numeric', 'boolean'].includes(column.data_type)
    );
    return preferred?.name ?? null;
  });
  let selectedTableKey = $derived.by(() =>
    selectedSchema && selectedTable ? `${selectedSchema}.${selectedTable}` : ''
  );
  let selectedRefreshSurfaceKey = $derived.by(() => {
    if (tablesSurface.kind === 'view' && selectedView) {
      return `view:${selectedView.id}`;
    }
    if (tablesSurface.kind === 'table' && selectedSchema && selectedTable) {
      return `table:${selectedSchema}.${selectedTable}`;
    }
    return null;
  });
  let selectedTableDisplayName = $derived.by(
    () => displayConfig?.tables[selectedTableKey]?.display_name ?? selectedTable
  );
  let selectedViewKey = $derived.by(() =>
    selectedView ? `view:${selectedView.id}` : ''
  );
  let selectedSurfaceVisibilityKey = $derived.by(() => {
    if (tablesSurface.kind === 'view') {
      return selectedViewKey;
    }
    if (tablesSurface.kind === 'table') {
      return selectedTableKey;
    }
    return '';
  });
  let selectedSurfaceDisplayName = $derived.by(() => {
    if (tablesSurface.kind === 'view') {
      return selectedView?.name ?? 'Saved view';
    }
    return selectedTableDisplayName;
  });
  let selectedViewId = $derived.by(() =>
    tablesSurface.kind === 'view' ? tablesSurface.viewId : null
  );
  let hasSurfaceSelection = $derived.by(() => {
    if (tablesSurface.kind === 'view') {
      return selectedView != null;
    }
    if (tablesSurface.kind === 'table') {
      return Boolean(selectedSchema && selectedTable);
    }
    return false;
  });
  let displayRows = $derived.by(() =>
    paginationMode === 'infinite' ? loadedRows : (queryResult?.rows ?? [])
  );
  let totalRows = $derived.by(() => queryResult?.total_rows ?? 0);
  let infiniteHasMore = $derived.by(() =>
    computeHasMore(loadedRows.filter((r) => !isSyntheticRow(r)).length, totalRows, rowCapState)
  );
  let brandingSettings = $derived.by(() =>
    parseBrandingSettings(appSettings, displayConfig)
  );
  let appearanceSettings = $derived.by(() => parseAppearanceSettings(appSettings));
  let densityClass = $derived.by(() =>
    appearanceSettings.rowDensity === 'compact'
      ? 'sk-density--compact'
      : 'sk-density--comfortable'
  );

  $effect(() => {
    refreshController.setSurface(selectedRefreshSurfaceKey);
  });

  function clearModeShortcut() {
    if (modeShortcutId !== null) {
      clearTimeout(modeShortcutId);
      modeShortcutId = null;
    }
    pendingModeShortcut = null;
  }

  function armModeShortcut() {
    clearModeShortcut();
    pendingModeShortcut = 'g';
    modeShortcutId = setTimeout(() => {
      pendingModeShortcut = null;
      modeShortcutId = null;
    }, 1000);
  }

  function applyUpdateStatus(status: UpdateStatus | null) {
    updateStatus = status;
    updateAvailable = status?.update_available ?? false;
  }

  async function refreshUpdateStatus() {
    try {
      applyUpdateStatus(await fetchUpdateStatus());
    } catch {
      // Update status is non-critical for the rest of the app shell.
    }
  }

  function getCurrentSurfaceSnapshot(): TablesSurface {
    if (tablesSurface.kind === 'view' && selectedView) {
      return { kind: 'view', viewId: selectedView.id };
    }
    return { kind: 'table' };
  }

  async function refreshViewsList() {
    try {
      savedViews = await fetchViews();
    } catch {
      // Leave the current in-memory view list alone if refresh fails.
    }
  }

  onMount(() => {
    const unsubscribe = refreshController.subscribe((value) => {
      refreshSnapshot = value;
    });

    return () => {
      unsubscribe();
      refreshController.destroy();
    };
  });

  onMount(() => {
    void (async () => {
      try {
        const status = await fetchStatus();
        if (status.mode === 'setup') {
          isSetup = true;
          return;
        }

        const [fetchedTables, views, config, settings] = await Promise.all([
          fetchTables(),
          fetchViews(),
          fetchDisplayConfig(),
          fetchSettings(),
        ]);
        tables = fetchedTables;
        savedViews = views;
        displayConfig = config;
        appSettings = settings;
        const dataSettings = parseDataSettings(settings);
        pageSize = dataSettings.pageSize;
        paginationMode = dataSettings.paginationMode;
        if (tables.length > 0) {
          await selectTable(tables[0]);
        }
        void refreshUpdateStatus();
        updateStatusPollId = setInterval(() => {
          void refreshUpdateStatus();
        }, 60_000);
      } catch (e) {
        error = e instanceof Error ? e.message : 'Failed to connect to database';
      } finally {
        loading = false;
      }
    })();

    return () => {
      if (updateStatusPollId === null) return;
      clearInterval(updateStatusPollId);
      updateStatusPollId = null;
    };
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
        if (tablesSurface.kind !== 'builder') {
          event.preventDefault();
          toggleSearch();
        }
        return;
      }

      if (isShortcut && key === 'f' && !inTextField) {
        if (tablesSurface.kind !== 'builder') {
          event.preventDefault();
          toggleFilters();
        }
        return;
      }

      if (event.key === 'Escape') {
        if (columnsOpen) {
          event.preventDefault();
          columnsOpen = false;
          void tick().then(() => columnsButtonEl?.focus());
          return;
        }

        if (searchVisible || searchQuery.length > 0) {
          event.preventDefault();
          handleSearchClear();
          void tick().then(() => searchButtonEl?.focus());
          return;
        }

        if (filtersVisible) {
          event.preventDefault();
          filtersVisible = false;
          void tick().then(() => filterButtonEl?.focus());
        }
      }

      if (!inTextField && !isShortcut && !event.shiftKey) {
        if (pendingModeShortcut === 'g') {
          if (key === 's') {
            event.preventDefault();
            setSidebarMode('settings');
            return;
          }

          if (key === 't') {
            event.preventDefault();
            setSidebarMode('tables');
            return;
          }

          clearModeShortcut();
        }

        if (key === 'g') {
          armModeShortcut();
        }
      }
    }

    window.addEventListener('keydown', handleKeydown);
    return () => {
      window.removeEventListener('keydown', handleKeydown);
      clearFilterDebounce();
      clearSearchDebounce();
      clearModeShortcut();
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

  function serializeSortState(nextSortState: SortState): string | undefined {
    if (nextSortState.length === 0) {
      return undefined;
    }

    return nextSortState
      .map(({ column, direction }) => `${column}:${direction}`)
      .join(',');
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
    scopeKey: string,
    tableColumns: ColumnInfo[],
  ): Record<string, boolean> {
    if (typeof localStorage === 'undefined') {
      return normalizeColumnVisibility(tableColumns, {});
    }

    const storageKey = `${COLUMN_VISIBILITY_KEY_PREFIX}${scopeKey}`;
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
    scopeKey: string,
    tableColumns: ColumnInfo[],
    visibility: Record<string, boolean>,
  ) {
    if (typeof localStorage === 'undefined') {
      return;
    }

    const storageKey = `${COLUMN_VISIBILITY_KEY_PREFIX}${scopeKey}`;
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

  function setSearchButtonEl(node: HTMLButtonElement | null) {
    searchButtonEl = node;
  }

  function setColumnsButtonEl(node: HTMLButtonElement | null) {
    columnsButtonEl = node;
  }

  function setFilterButtonEl(node: HTMLButtonElement | null) {
    filterButtonEl = node;
  }

  function clearLastUsedSaveDebounce() {
    if (lastUsedSaveId !== null) {
      clearTimeout(lastUsedSaveId);
      lastUsedSaveId = null;
    }
  }

  function scheduleLastUsedSave(
    schema: string,
    table: string,
    nextSortState: SortState,
    nextFilters: FilterState,
    nextSearch: string,
  ) {
    clearLastUsedSaveDebounce();
    lastUsedSaveId = setTimeout(() => {
      lastUsedSaveId = null;
      const sortCols: SortColumn[] = nextSortState.map(({ column, direction }) => ({
        col: column,
        dir: direction,
      }));
      void saveLastUsedState(schema, table, {
        sort_columns: sortCols,
        filters: nextFilters,
        search_term: nextSearch || null,
        page_size: pageSize,
      }).catch(() => {
        // Non-fatal — last-used save should never break the UI
      });
    }, 500);
  }

  function buildRowsParams(
    page: number,
    nextSortState: SortState = sortState,
    nextFilters: FilterState = filters,
    nextSearchTerm: string = searchTerm,
  ): FetchRowsParams {
    const params: FetchRowsParams = { page, page_size: pageSize };
    const sort = serializeSortState(nextSortState);
    if (sort != null) {
      params.sort = sort;
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

  async function selectTable(table: TableInfo) {
    const myRequest = ++selectRequestId;
    tablesSurface = { kind: 'table' };
    selectedSchema = table.schema;
    selectedTable = table.name;
    selectedView = null;
    tableError = null;
    tableLoading = true;
    currentPage = 1;
    filtersVisible = false;
    columnsOpen = false;
    clearFilterDebounce();
    clearLastUsedSaveDebounce();
    resetSearchState();
    const visibilityKey = `${table.schema}.${table.name}`;

    let initialSortState: SortState = [];
    let initialFilters: FilterState = {};
    let initialSearch = '';

    try {
      const [cols, lastUsed] = await Promise.all([
        fetchColumns(table.schema, table.name),
        !isSetup ? fetchLastUsedState(table.schema, table.name) : Promise.resolve(null),
      ]);
      if (myRequest !== selectRequestId) return;

      if (lastUsed) {
        initialSortState = lastUsed.sort_columns.map(({ col, dir }) => ({
          column: col,
          direction: dir,
        }));
        initialFilters = lastUsed.filters;
        initialSearch = lastUsed.search_term ?? '';
        if (lastUsed.page_size != null && isPageSizePreference(lastUsed.page_size)) {
          pageSize = lastUsed.page_size;
        } else {
          const globalPageSize = parseDataSettings(appSettings).pageSize;
          pageSize = globalPageSize;
        }
      } else {
        pageSize = parseDataSettings(appSettings).pageSize;
      }

      columns = cols;
      columnVisibility = loadColumnVisibility(visibilityKey, cols);
      sortState = initialSortState;
      filters = initialFilters;
      searchTerm = initialSearch;

      const scrollInit = resetState();
      loadedRows = scrollInit.rows;
      lastLoadedPage = scrollInit.lastLoadedPage;
      rowCapState = scrollInit.capState;
      fetchingMore = false;
      appendError = false;
      resetSignal++;

      const result = await fetchRows(
        table.schema,
        table.name,
        buildRowsParams(1, initialSortState, initialFilters, initialSearch),
      );
      if (myRequest !== selectRequestId) return;

      queryResult = result;
      if (paginationMode === 'infinite') {
        const nextState = appendBatch(
          { rows: [], loadedCount: 0, lastLoadedPage: 0, capState: 'none' },
          result.rows,
          1,
          result.total_rows,
        );
        loadedRows = nextState.rows;
        lastLoadedPage = nextState.lastLoadedPage;
        rowCapState = nextState.capState;
      }
      refreshController.markRefreshed();
    } catch (e) {
      if (myRequest !== selectRequestId) return;
      tableError = e instanceof Error ? e.message : 'Failed to load table';
    } finally {
      if (myRequest === selectRequestId) tableLoading = false;
    }
  }

  async function openSavedView(view: SavedViewSummary) {
    const myRequest = ++selectRequestId;
    tablesSurface = { kind: 'view', viewId: view.id };
    tableError = null;
    tableLoading = true;
    currentPage = 1;
    filtersVisible = false;
    columnsOpen = false;
    clearFilterDebounce();
    clearLastUsedSaveDebounce();
    resetSearchState();

    try {
      const scrollInit = resetState();
      loadedRows = scrollInit.rows;
      lastLoadedPage = scrollInit.lastLoadedPage;
      rowCapState = scrollInit.capState;
      fetchingMore = false;
      appendError = false;
      resetSignal++;

      const [definition, result] = await Promise.all([
        fetchView(view.id),
        fetchViewRows(view.id, buildRowsParams(1, [], {}, '')),
      ]);
      if (myRequest !== selectRequestId) return;

      selectedView = definition;
      columns = result.columns;
      columnVisibility = loadColumnVisibility(`view:${definition.id}`, result.columns);
      queryResult = result;
      sortState = [];
      filters = {};
      searchTerm = '';
      if (paginationMode === 'infinite') {
        const nextState = appendBatch(
          { rows: [], loadedCount: 0, lastLoadedPage: 0, capState: 'none' },
          result.rows,
          1,
          result.total_rows,
        );
        loadedRows = nextState.rows;
        lastLoadedPage = nextState.lastLoadedPage;
        rowCapState = nextState.capState;
      }
      refreshController.markRefreshed();
    } catch (e) {
      if (myRequest !== selectRequestId) return;
      tableError = e instanceof Error ? e.message : 'Failed to load saved view';
    } finally {
      if (myRequest === selectRequestId) tableLoading = false;
    }
  }

  async function loadRows(
    page: number,
    nextSortState: SortState = sortState,
    nextFilters: FilterState = filters,
    nextSearchTerm: string = searchTerm,
  ) {
    const myRequest = ++selectRequestId;
    tableError = null;
    tableLoading = true;
    try {
      const params = buildRowsParams(page, nextSortState, nextFilters, nextSearchTerm);
      const result =
        tablesSurface.kind === 'view' && selectedView
          ? await fetchViewRows(selectedView.id, params)
          : tablesSurface.kind === 'table' && selectedSchema && selectedTable
            ? await fetchRows(selectedSchema, selectedTable, params)
            : null;

      if (myRequest !== selectRequestId || result == null) return;
      queryResult = result;
      currentPage = page;
      if (paginationMode === 'infinite') {
        const nextState = appendBatch(
          { rows: loadedRows.filter((r) => !isSyntheticRow(r)), loadedCount: loadedRows.filter((r) => !isSyntheticRow(r)).length, lastLoadedPage: page - 1, capState: rowCapState },
          result.rows,
          page,
          result.total_rows,
        );
        loadedRows = nextState.rows;
        lastLoadedPage = nextState.lastLoadedPage;
        rowCapState = nextState.capState;
      }
      if (tablesSurface.kind === 'view' && selectedView) {
        columns = result.columns;
        columnVisibility = normalizeColumnVisibility(result.columns, columnVisibility);
      }
      refreshController.markRefreshed();
    } catch (e) {
      if (myRequest !== selectRequestId) return;
      tableError = e instanceof Error ? e.message : 'Failed to load rows';
    } finally {
      if (myRequest === selectRequestId) tableLoading = false;
    }
  }

  async function resetAndLoadRows(
    nextSort: SortState = sortState,
    nextFilters: FilterState = filters,
    nextSearch: string = searchTerm,
  ) {
    const scrollInit = resetState();
    loadedRows = scrollInit.rows;
    lastLoadedPage = scrollInit.lastLoadedPage;
    rowCapState = scrollInit.capState;
    fetchingMore = false;
    appendError = false;
    resetSignal++;
    await loadRows(1, nextSort, nextFilters, nextSearch);
  }

  async function fetchNextPage(page: number): Promise<QueryResult | null> {
    const params = buildRowsParams(page);
    if (tablesSurface.kind === 'view' && selectedView) {
      return fetchViewRows(selectedView.id, params);
    }
    if (tablesSurface.kind === 'table' && selectedSchema && selectedTable) {
      return fetchRows(selectedSchema, selectedTable, params);
    }
    return null;
  }

  async function loadMoreRows() {
    if (fetchingMore || !infiniteHasMore) return;
    const nextPage = lastLoadedPage + 1;
    const myRequest = ++selectRequestId;

    const colNames = visibleColumns.map((c) => c.name);
    loadedRows = [
      ...loadedRows.filter((r) => !isSyntheticRow(r)),
      ...makeSyntheticSkeletonRows(3, colNames),
    ];
    fetchingMore = true;

    let result: QueryResult | null = null;
    let failed = false;

    try {
      result = await fetchNextPage(nextPage);
    } catch {
      // First failure — auto-retry once
      try {
        result = await fetchNextPage(nextPage);
      } catch {
        failed = true;
      }
    }

    if (myRequest !== selectRequestId) {
      loadedRows = loadedRows.filter((r) => !isSyntheticRow(r));
      fetchingMore = false;
      return;
    }

    if (failed || result == null) {
      const cleanRows = loadedRows.filter((r) => !isSyntheticRow(r));
      loadedRows = [...cleanRows, makeInlineErrorRow(colNames)];
      appendError = true;
      fetchingMore = false;
      return;
    }

    const nextState = appendBatch(
      {
        rows: loadedRows.filter((r) => !isSyntheticRow(r)),
        loadedCount: loadedRows.filter((r) => !isSyntheticRow(r)).length,
        lastLoadedPage,
        capState: rowCapState,
      },
      result.rows,
      nextPage,
      result.total_rows,
    );
    loadedRows = nextState.rows;
    lastLoadedPage = nextState.lastLoadedPage;
    rowCapState = nextState.capState;
    queryResult = result;
    fetchingMore = false;
  }

  async function handlePageSizeChange(size: PageSizePreference) {
    pageSize = size;
    const entries = buildDataSettingsEntries({ pageSize: size, paginationMode });
    void saveSettings(entries).catch(() => {});
    appSettings = { ...appSettings, ...entries };
    if (paginationMode === 'infinite') {
      await resetAndLoadRows();
    } else {
      await loadRows(1);
    }
  }

  async function goToPage(page: number) {
    clearFilterDebounce();
    clearSearchDebounce();
    await loadRows(page);
  }

  function handleSortChange(nextSortState: SortState) {
    clearFilterDebounce();
    clearSearchDebounce();
    sortState = nextSortState;
    void resetAndLoadRows(nextSortState);
    if (tablesSurface.kind === 'table' && selectedSchema && selectedTable) {
      scheduleLastUsedSave(selectedSchema, selectedTable, nextSortState, filters, searchTerm);
    }
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
      void resetAndLoadRows(sortState, nextFilters);
      filterDebounceId = null;
    }, 300);
    if (tablesSurface.kind === 'table' && selectedSchema && selectedTable) {
      scheduleLastUsedSave(selectedSchema, selectedTable, sortState, nextFilters, searchTerm);
    }
  }

  function scheduleSearchReload() {
    clearSearchDebounce();
    clearFilterDebounce();
    if (!hasSurfaceSelection) return;

    searchDebounceId = setTimeout(() => {
      void resetAndLoadRows();
      searchDebounceId = null;
    }, 300);
    if (tablesSurface.kind === 'table' && selectedSchema && selectedTable) {
      scheduleLastUsedSave(selectedSchema, selectedTable, sortState, filters, searchTerm);
    }
  }

  function handleSearchInput(event: Event) {
    searchTerm = (event.currentTarget as HTMLInputElement).value;
    scheduleSearchReload();
  }

  function handleSearchClear() {
    clearFilterDebounce();
    resetSearchState();
    if (!hasSurfaceSelection) {
      return;
    }

    void resetAndLoadRows(sortState, filters, '');
  }

  function handleToggleColumnVisibility(columnName: string, visible: boolean) {
    if (!selectedSurfaceVisibilityKey) return;

    const nextVisibility = normalizeColumnVisibility(columns, {
      ...columnVisibility,
      [columnName]: visible,
    });
    columnVisibility = nextVisibility;
    persistColumnVisibility(selectedSurfaceVisibilityKey, columns, nextVisibility);
  }

  function handleShowAllColumns() {
    if (!selectedSurfaceVisibilityKey) return;

    const nextVisibility = normalizeColumnVisibility(
      columns,
      Object.fromEntries(columns.map((column) => [column.name, true])) as Record<
        string,
        boolean
      >
    );
    columnVisibility = nextVisibility;
    persistColumnVisibility(selectedSurfaceVisibilityKey, columns, nextVisibility);
  }

  function exportCsv() {
    const params = buildRowsParams(1);
    if (tablesSurface.kind === 'view' && selectedView) {
      window.open(buildViewCsvUrl(selectedView.id, params), '_blank');
      return;
    }

    if (!selectedTable || !selectedSchema) return;

    const searchParams = new URLSearchParams();
    if (params.sort) searchParams.set('sort', params.sort);
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

  async function handleRefreshNow() {
    try {
      await refreshController.refreshNow();
    } catch (e) {
      tableError = e instanceof Error ? e.message : 'Failed to refresh rows';
    }
  }

  async function handleSaveBranding(nextBranding: BrandingSettings) {
    const entries = buildBrandingSettingsEntries(nextBranding);
    await saveSettings(entries);
    appSettings = {
      ...appSettings,
      ...entries,
    };
    try {
      displayConfig = await fetchDisplayConfig();
    } catch {
      // Save succeeded; display-config refresh failure is non-fatal.
    }
  }

  async function handleSaveAppearance(nextAppearance: AppearanceSettings) {
    const entries = buildAppearanceSettingsEntries(nextAppearance);
    await saveSettings(entries);
    appSettings = {
      ...appSettings,
      ...entries,
    };
  }

  async function handlePaginationModeChange(mode: PaginationMode) {
    paginationMode = mode;
    const entries = buildDataSettingsEntries({ pageSize, paginationMode: mode });
    void saveSettings(entries).catch(() => {});
    appSettings = { ...appSettings, ...entries };
    if (mode === 'infinite') {
      await resetAndLoadRows();
    }
  }

  function openBuilder(
    draft: ViewDraft,
    sourceLabel = '',
    returnTarget: TablesSurface = getCurrentSurfaceSnapshot(),
  ) {
    builderDraft = cloneDraft(draft);
    builderDraftLive = null;
    builderSourceLabel = sourceLabel;
    builderReturnTarget = returnTarget;
    tablesSurface = { kind: 'builder' };
    columnsOpen = false;
    searchVisible = false;
    filtersVisible = false;
    tableError = null;
  }

  function forceCreateView() {
    pendingCreateView = false;
    const baseTable =
      tables.find((table) => table.schema === selectedSchema && table.name === selectedTable) ??
      tables[0];
    if (!baseTable) return;

    openBuilder(
      {
        name: '',
        base_schema: baseTable.schema,
        base_table: baseTable.name,
        definition_version: 2,
        columns: [],
        filters: {},
        sources: [],
        grouping: null,
        ranking: null,
        template: null,
      },
      selectedTableDisplayName || `${baseTable.schema}.${baseTable.name}`,
    );
  }

  function handleCreateView() {
    const live = builderDraftLive ?? builderDraft;
    if (live && (live.columns.length > 0 || (live.sources?.length ?? 0) > 0 || live.grouping != null || live.ranking != null)) {
      pendingCreateView = true;
      return;
    }
    forceCreateView();
  }

  async function handleDuplicateView(view: SavedViewSummary) {
    const returnTarget = getCurrentSurfaceSnapshot();
    const definition =
      selectedView?.id === view.id ? selectedView : await fetchView(view.id);
    openBuilder(draftFromView(definition), view.name, returnTarget);
  }

  function handleCopyToEdit() {
    if (!selectedView) return;
    openBuilder(
      draftFromView(selectedView),
      selectedView.name,
      { kind: 'view', viewId: selectedView.id },
    );
  }

  async function handleBuilderSaved(summary: SavedViewSummary) {
    savedViews = [...savedViews.filter((view) => view.id !== summary.id), summary].sort((a, b) =>
      a.name.localeCompare(b.name)
    );
    builderDraft = null;
    builderDraftLive = null;
    builderSourceLabel = '';
    builderReturnTarget = { kind: 'table' };
    await refreshViewsList();
    await openSavedView(summary);
  }

  async function handleCancelBuilder() {
    const target = builderReturnTarget;
    builderDraft = null;
    builderDraftLive = null;
    builderSourceLabel = '';
    builderReturnTarget = { kind: 'table' };

    if (target.kind === 'view' && selectedView?.id === target.viewId) {
      tablesSurface = { kind: 'view', viewId: target.viewId };
      return;
    }

    if (target.kind === 'view') {
      const targetView = savedViews.find((view) => view.id === target.viewId);
      if (targetView) {
        await openSavedView(targetView);
        return;
      }
    }

    if (selectedSchema && selectedTable) {
      tablesSurface = { kind: 'table' };
      return;
    }

    if (tables.length > 0) {
      await selectTable(tables[0]);
    }
  }

  async function handleRenameSavedView(view: SavedViewSummary, name: string) {
    const renamed = await renameView(view.id, name);
    const nextName = renamed?.name ?? name;
    savedViews = savedViews
      .map((candidate) =>
        candidate.id === view.id ? { ...candidate, name: nextName } : candidate
      )
      .sort((a, b) => a.name.localeCompare(b.name));
    if (selectedView?.id === view.id) {
      selectedView = {
        ...selectedView,
        name: nextName,
      };
    }
    await refreshViewsList();
  }

  async function handleDeleteSavedView(view: SavedViewSummary) {
    try {
      await deleteView(view.id);
    } catch (e) {
      tableError = e instanceof Error ? e.message : 'Failed to delete view';
      return;
    }
    savedViews = savedViews.filter((candidate) => candidate.id !== view.id);

    if (builderReturnTarget.kind === 'view' && builderReturnTarget.viewId === view.id) {
      builderReturnTarget = { kind: 'table' };
    }

    if (selectedView?.id === view.id || (tablesSurface.kind === 'view' && tablesSurface.viewId === view.id)) {
      const fallback =
        tables.find(
          (table) => table.schema === view.base_schema && table.name === view.base_table
        ) ?? tables[0];
      selectedView = null;
      if (fallback) {
        await selectTable(fallback);
      } else {
        tablesSurface = { kind: 'table' };
        queryResult = null;
        columns = [];
      }
    }
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
      onToggle={() => (sidebarCollapsed = !sidebarCollapsed)}
      title="SeeKi"
      subtitle=""
      {updateAvailable}
      showSettingsBadge={updateAvailable}
      onSettingsClick={() => (settingsOpen = true)}
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
  <SettingsPanel
    bind:open={settingsOpen}
    initialStatus={updateStatus}
    onStatusChange={(s) => {
      applyUpdateStatus(s);
    }}
  />
{:else}
  <div class={`layout ${densityClass}`}>
    <Sidebar
      bind:collapsed={sidebarCollapsed}
      onToggle={() => (sidebarCollapsed = !sidebarCollapsed)}
      onSelectMode={setSidebarMode}
      title={displayConfig?.branding?.title ?? 'SeeKi'}
      subtitle={displayConfig?.branding?.subtitle ?? ''}
      {updateAvailable}
      mode={$sidebarMode}
      showSettingsBadge={updateAvailable}
      showModeSwitch={true}
    >
      {#if !sidebarCollapsed}
        {#key $sidebarMode}
          <div class="sidebar-panel-enter">
            {#if $sidebarMode === 'tables'}
              <DataPanels
                {tables}
                selectedSchema={tablesSurface.kind === 'table' ? selectedSchema : ''}
                selectedTable={tablesSurface.kind === 'table' ? selectedTable : ''}
                onSelectTable={selectTable}
                views={savedViews}
                activeViewId={selectedViewId}
                viewsDisabled={tables.length === 0}
                onSelectView={openSavedView}
                onCreateView={handleCreateView}
                onRenameView={(view, name) => void handleRenameSavedView(view, name)}
                onDeleteView={(view) => void handleDeleteSavedView(view)}
                onDuplicateView={(view) => void handleDuplicateView(view)}
              />
            {:else}
              <SettingsNav showUpdateBadge={updateAvailable} />
            {/if}
          </div>
        {/key}
      {/if}
    </Sidebar>
    {#key $sidebarMode}
      <div class="main-panel-enter">
        {#if $sidebarMode === 'tables'}
          {#if tablesSurface.kind === 'builder' && builderDraft}
            <main class="main">
              <ViewBuilder
                tables={tables}
                initialDraft={builderDraft}
                sourceLabel={builderSourceLabel}
                onCancel={handleCancelBuilder}
                onSaved={handleBuilderSaved}
                onDraftChange={(d) => { builderDraftLive = { ...d }; }}
              />
            </main>
          {:else}
            <main class="main">
              <div class="table-panel">
                <TableHeader
                  tableName={selectedSurfaceDisplayName}
                  rowCount={queryResult?.total_rows ?? 0}
                />
                {#if tablesSurface.kind === 'view' && selectedView}
                  <div class="view-toolbar">
                    <div class="view-meta">
                      <span class="view-pill">Read-only saved view</span>
                      <span>Base table: {selectedView.base_schema}.{selectedView.base_table}</span>
                    </div>
                    <div class="view-toolbar-actions">
                      <button type="button" class="view-action" onclick={handleCopyToEdit}>
                        Copy to edit
                      </button>
                      <button
                        type="button"
                        class="view-action view-action--danger"
                        onclick={() => {
                          if (selectedView) {
                            void handleDeleteSavedView(selectedView);
                          }
                        }}
                      >
                        Delete view
                      </button>
                    </div>
                  </div>
                {/if}
              </div>
              <GridRefreshToolbar
                surfaceKey={selectedRefreshSurfaceKey ?? ''}
                intervalMs={refreshSnapshot.intervalMs}
                lastRefreshedAt={refreshSnapshot.lastRefreshedAt}
                refreshing={refreshSnapshot.inFlight || tableLoading}
                disabled={!hasSurfaceSelection || tablesSurface.kind === 'builder' || tableLoading}
                onRefreshNow={() => void handleRefreshNow()}
                onIntervalChange={(intervalMs) => refreshController.setIntervalMs(intervalMs)}
              />
              {#if tableError}
                <div class="table-error-banner">
                  <span>{tableError}</span>
                  <button class="dismiss-btn" onclick={() => (tableError = null)}>Dismiss</button>
                </div>
              {/if}
              <QuickStatsBar
                totalRows={queryResult?.total_rows ?? 0}
                rows={queryResult?.rows ?? []}
                {visibleColumns}
                {focusedTextColumnName}
              />
              {#if paginationMode === 'infinite' && rowCapState !== 'none'}
                <RowCapWarning
                  capState={rowCapState}
                  loadedCount={loadedRows.filter((r) => !isSyntheticRow(r)).length}
                />
              {/if}
              <div class="grid-area">
                <div class="grid-shell">
                  <div class="grid-content" class:stale={tableLoading && paginationMode === 'infinite'}>
                    <DataGrid
                      columns={visibleColumns}
                      rows={displayRows}
                      dateFormat={appearanceSettings.dateFormat}
                      {sortState}
                      {filters}
                      {filtersVisible}
                      {fetchingMore}
                      {appendError}
                      {resetSignal}
                      onSortChange={handleSortChange}
                      onFilterChange={handleFilterChange}
                      onNearBottom={() => { if (paginationMode === 'infinite') void loadMoreRows(); }}
                      onRetryAppend={() => { appendError = false; void loadMoreRows(); }}
                    />
                  </div>
                  {#if hasSurfaceSelection}
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
                      hasTable={hasSurfaceSelection}
                      disabled={tableLoading}
                      sortState={sortState}
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
                      onSearchButtonRef={setSearchButtonEl}
                      onColumnsButtonRef={setColumnsButtonEl}
                      onFilterButtonRef={setFilterButtonEl}
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
                mode={paginationMode}
                total={queryResult?.total_rows ?? 0}
                loadedCount={loadedRows.filter((r) => !isSyntheticRow(r)).length}
                start={
                  queryResult && queryResult.total_rows > 0
                    ? (queryResult.page - 1) * queryResult.page_size + 1
                    : 0
                }
                end={
                  queryResult && queryResult.total_rows > 0
                    ? Math.min(queryResult.page * queryResult.page_size, queryResult.total_rows)
                    : 0
                }
                page={queryResult?.page ?? 1}
                totalPages={
                  queryResult
                    ? Math.max(1, Math.ceil(queryResult.total_rows / queryResult.page_size))
                    : 1
                }
                {pageSize}
                loading={tableLoading}
                onPageChange={goToPage}
                onPageSizeChange={handlePageSizeChange}
              />
            </main>
          {/if}
        {:else}
          <main class="main settings-main">
            <SettingsContent
              branding={brandingSettings}
              appearance={appearanceSettings}
              {paginationMode}
              {updateStatus}
              onSaveBranding={handleSaveBranding}
              onSaveAppearance={handleSaveAppearance}
              onPaginationModeChange={handlePaginationModeChange}
              onUpdateStatusChange={(s) => {
                applyUpdateStatus(s);
              }}
            />
          </main>
        {/if}
      </div>
    {/key}
  </div>
  <SettingsPanel
    bind:open={settingsOpen}
    initialStatus={updateStatus}
    onStatusChange={(s) => {
      applyUpdateStatus(s);
    }}
  />
{/if}

{#if pendingCreateView}
  <div
    class="draft-guard-backdrop"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) pendingCreateView = false; }}
  >
    <div class="draft-guard-card" role="dialog" aria-modal="true" aria-label="Unsaved draft">
      <p class="draft-guard-title">You have an unsaved view draft</p>
      <p class="draft-guard-detail">Creating a new view will discard your current draft including columns, joins, grouping, and ranking. Continue?</p>
      <div class="draft-guard-actions">
        <button type="button" class="draft-guard-btn draft-guard-btn-secondary" onclick={() => pendingCreateView = false}>Keep editing</button>
        <button type="button" class="draft-guard-btn draft-guard-btn-danger" onclick={forceCreateView}>Discard &amp; create new</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .layout {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .sidebar-panel-enter {
    animation: sk-fade-in 180ms ease-out;
  }

  .tables-sidebar {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
  }

  .main-panel-enter {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    animation: sk-fade-in 200ms ease-out;
  }

  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .settings-main {
    overflow: hidden;
  }

  .table-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-lg) var(--sk-space-2xl) 0;
  }

  .view-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-md);
    padding-bottom: var(--sk-space-sm);
    border-bottom: 1px solid var(--sk-border-light);
  }

  .view-meta,
  .view-toolbar-actions {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  .view-meta {
    color: var(--sk-secondary-strong);
    font-size: var(--sk-font-size-sm);
  }

  .view-pill {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    background: rgba(0, 169, 165, 0.1);
    color: var(--sk-accent);
    padding: 4px 10px;
    font-weight: 600;
  }

  .view-action {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.78);
    color: var(--sk-secondary-strong);
    padding: 8px 12px;
    font: inherit;
    cursor: pointer;
  }

  .view-action--danger:hover {
    color: #b91c1c;
    border-color: rgba(185, 28, 28, 0.28);
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
    transition: opacity 150ms ease-out;
  }

  .grid-content.stale {
    opacity: 0.55;
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

  @media (max-width: 900px) {
    .table-panel,
    .grid-area {
      padding-left: var(--sk-space-lg);
      padding-right: var(--sk-space-lg);
    }

    .view-toolbar {
      flex-direction: column;
      align-items: flex-start;
    }
  }

  .draft-guard-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sk-space-lg);
    z-index: 1000;
    animation: dg-fade 120ms ease-out;
  }

  .draft-guard-card {
    max-width: 420px;
    width: 100%;
    background: var(--sk-bg, rgba(255, 255, 255, 0.98));
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-lg);
    box-shadow: 0 20px 60px rgba(15, 23, 42, 0.35);
    animation: dg-pop 140ms ease-out;
  }

  .draft-guard-title {
    margin: 0 0 var(--sk-space-sm);
    font-weight: 600;
    color: var(--sk-text);
  }

  .draft-guard-detail {
    margin: 0 0 var(--sk-space-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-muted);
  }

  .draft-guard-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sk-space-sm);
  }

  .draft-guard-btn {
    display: inline-flex;
    align-items: center;
    border-radius: var(--sk-radius-md);
    font: inherit;
    cursor: pointer;
    padding: 7px 14px;
    font-size: var(--sk-font-size-body);
  }

  .draft-guard-btn-secondary {
    border: 1px solid var(--sk-border-light);
    background: transparent;
    color: var(--sk-text);
  }

  .draft-guard-btn-secondary:hover { background: rgba(47, 72, 88, 0.04); }

  .draft-guard-btn-danger {
    border: 1px solid rgba(181, 71, 71, 0.3);
    background: rgba(181, 71, 71, 0.08);
    color: #b54747;
  }

  .draft-guard-btn-danger:hover { background: rgba(181, 71, 71, 0.16); }

  @keyframes dg-fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes dg-pop {
    from { transform: scale(0.95); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }
</style>
