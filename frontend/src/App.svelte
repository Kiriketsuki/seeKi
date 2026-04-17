<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import SettingsNav from './components/SettingsNav.svelte';
  import SettingsContent from './components/SettingsContent.svelte';
  import TableList from './components/TableList.svelte';
  import ViewList from './components/ViewList.svelte';
  import ViewBuilder from './components/ViewBuilder.svelte';
  import ActionDock from './components/ActionDock.svelte';
  import TableHeader from './components/TableHeader.svelte';
  import DataGrid from './components/DataGrid.svelte';
  import StatusBar from './components/StatusBar.svelte';
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
    ViewDraft,
  } from './lib/types';
  import { COLUMN_VISIBILITY_KEY_PREFIX, SIDEBAR_COLLAPSED_KEY } from './lib/constants';
  import { sidebarMode } from './lib/stores';
  import {
    buildAppearanceSettingsEntries,
    buildBrandingSettingsEntries,
    parseAppearanceSettings,
    parseBrandingSettings,
  } from './lib/settings';

  function setSidebarMode(mode: SidebarMode) {
    sidebarMode.set(mode);
  }

  function cloneDraft(source: ViewDraft): ViewDraft {
    return {
      name: source.name,
      base_schema: source.base_schema,
      base_table: source.base_table,
      columns: source.columns.map((column) => ({ ...column })),
      filters: { ...source.filters },
    };
  }

  function draftFromView(view: SavedViewDefinition, name = `${view.name} copy`): ViewDraft {
    return {
      name,
      base_schema: view.base_schema,
      base_table: view.base_table,
      columns: view.columns.map((column) => ({ ...column })),
      filters: { ...view.filters },
    };
  }

  let tables: TableInfo[] = $state([]);
  let savedViews: SavedViewSummary[] = $state([]);
  let tablesSurface: TablesSurface = $state({ kind: 'table' });
  let builderDraft: ViewDraft | null = $state(null);
  let builderSourceLabel = $state('');
  let builderReturnTarget: TablesSurface = $state({ kind: 'table' });
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
  let filterDebounceId: ReturnType<typeof setTimeout> | null = null;
  let searchDebounceId: ReturnType<typeof setTimeout> | null = null;
  let lastUsedSaveId: ReturnType<typeof setTimeout> | null = null;
  let modeShortcutId: ReturnType<typeof setTimeout> | null = null;
  let updateStatusPollId: ReturnType<typeof setInterval> | null = null;
  let pendingModeShortcut: 'g' | null = null;
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
  let brandingSettings = $derived.by(() =>
    parseBrandingSettings(appSettings, displayConfig)
  );
  let appearanceSettings = $derived.by(() => parseAppearanceSettings(appSettings));
  let densityClass = $derived.by(() =>
    appearanceSettings.rowDensity === 'compact'
      ? 'sk-density--compact'
      : 'sk-density--comfortable'
  );

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
    const params: FetchRowsParams = { page };
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
      }

      const result = await fetchRows(
        table.schema,
        table.name,
        buildRowsParams(1, initialSortState, initialFilters, initialSearch),
      );
      if (myRequest !== selectRequestId) return;

      columns = cols;
      columnVisibility = loadColumnVisibility(visibilityKey, cols);
      queryResult = result;
      sortState = initialSortState;
      filters = initialFilters;
      searchTerm = initialSearch;
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
      if (tablesSurface.kind === 'view' && selectedView) {
        columns = result.columns;
        columnVisibility = normalizeColumnVisibility(result.columns, columnVisibility);
      }
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

  function handleSortChange(nextSortState: SortState) {
    clearFilterDebounce();
    clearSearchDebounce();
    sortState = nextSortState;
    void loadRows(1, nextSortState);
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
      void loadRows(1, sortState, nextFilters);
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
      void loadRows(1);
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

    void loadRows(1, sortState, filters, '');
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

  function openBuilder(
    draft: ViewDraft,
    sourceLabel = '',
    returnTarget: TablesSurface = getCurrentSurfaceSnapshot(),
  ) {
    builderDraft = cloneDraft(draft);
    builderSourceLabel = sourceLabel;
    builderReturnTarget = returnTarget;
    tablesSurface = { kind: 'builder' };
    columnsOpen = false;
    searchVisible = false;
    filtersVisible = false;
    tableError = null;
  }

  function handleCreateView() {
    const baseTable =
      tables.find((table) => table.schema === selectedSchema && table.name === selectedTable) ??
      tables[0];
    if (!baseTable) return;

    openBuilder(
      {
        name: '',
        base_schema: baseTable.schema,
        base_table: baseTable.name,
        columns: [],
        filters: {},
      },
      selectedTableDisplayName || `${baseTable.schema}.${baseTable.name}`,
    );
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
    builderSourceLabel = '';
    builderReturnTarget = { kind: 'table' };
    await refreshViewsList();
    await openSavedView(summary);
  }

  async function handleCancelBuilder() {
    const target = builderReturnTarget;
    builderDraft = null;
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
              <div class="tables-sidebar">
                <TableList
                  {tables}
                  selectedSchema={tablesSurface.kind === 'table' ? selectedSchema : ''}
                  selectedTable={tablesSurface.kind === 'table' ? selectedTable : ''}
                  onSelect={selectTable}
                />
                <ViewList
                  views={savedViews}
                  activeViewId={selectedViewId}
                  disabled={tables.length === 0}
                  onSelect={openSavedView}
                  onCreate={handleCreateView}
                  onRename={(view, name) => void handleRenameSavedView(view, name)}
                  onDelete={(view) => void handleDeleteSavedView(view)}
                  onDuplicate={(view) => void handleDuplicateView(view)}
                />
              </div>
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
              {#if tableError}
                <div class="table-error-banner">
                  <span>{tableError}</span>
                  <button class="dismiss-btn" onclick={() => (tableError = null)}>Dismiss</button>
                </div>
              {/if}
              <div class="grid-area">
                <div class="grid-shell">
                  <div class="grid-content">
                    <DataGrid
                      columns={visibleColumns}
                      rows={queryResult?.rows ?? []}
                      dateFormat={appearanceSettings.dateFormat}
                      {sortState}
                      {filters}
                      {filtersVisible}
                      onSortChange={handleSortChange}
                      onFilterChange={handleFilterChange}
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
                total={queryResult?.total_rows ?? 0}
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
                loading={tableLoading}
                onPageChange={goToPage}
              />
            </main>
          {/if}
        {:else}
          <main class="main settings-main">
            <SettingsContent
              branding={brandingSettings}
              appearance={appearanceSettings}
              {updateStatus}
              onSaveBranding={handleSaveBranding}
              onSaveAppearance={handleSaveAppearance}
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
</style>
