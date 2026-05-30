import { get, writable } from 'svelte/store';
import {
  DATA_PANELS_LAYOUT_KEY,
  GRID_REFRESH_INTERVALS,
  GRID_REFRESH_KEY_PREFIX,
} from './constants';
import { getColumnDisplayName } from './data-grid';
import type { ColumnInfo, SettingsSection, SidebarMode } from './types';

export const sidebarMode = writable<SidebarMode>('tables');
export const activeSettingsSection = writable<SettingsSection>('updates');

type StorageLike = Pick<Storage, 'getItem' | 'setItem'>;

function getBrowserStorage(): StorageLike | null {
  if (typeof localStorage === 'undefined') {
    return null;
  }

  return localStorage;
}

export type DataPanelId = 'tables' | 'views';

export interface DataPanelLayout {
  order: [DataPanelId, DataPanelId];
  sizes: Record<DataPanelId, number>;
  collapsed: Record<DataPanelId, boolean>;
}

const DATA_PANEL_IDS: DataPanelId[] = ['tables', 'views'];
const DATA_PANEL_MIN_SIZE = 20;
const DATA_PANEL_MAX_SIZE = 80;

export const DEFAULT_DATA_PANEL_LAYOUT: DataPanelLayout = {
  order: ['tables', 'views'],
  sizes: {
    tables: 60,
    views: 40,
  },
  collapsed: {
    tables: false,
    views: false,
  },
};

function isDataPanelId(value: unknown): value is DataPanelId {
  return value === 'tables' || value === 'views';
}

function clampDataPanelSize(value: number): number {
  return Math.min(DATA_PANEL_MAX_SIZE, Math.max(DATA_PANEL_MIN_SIZE, Math.round(value)));
}

export function normalizeDataPanelLayout(
  value: Partial<DataPanelLayout> | null | undefined,
): DataPanelLayout {
  const rawOrder = Array.isArray(value?.order)
    ? value.order.filter(isDataPanelId)
    : [];
  const uniqueOrder = Array.from(new Set(rawOrder));
  const order =
    uniqueOrder.length === 2
      ? ([uniqueOrder[0], uniqueOrder[1]] as [DataPanelId, DataPanelId])
      : DEFAULT_DATA_PANEL_LAYOUT.order;

  const [topPanel, bottomPanel] = order;
  const topPanelSize =
    typeof value?.sizes?.[topPanel] === 'number'
      ? clampDataPanelSize(value.sizes[topPanel])
      : DEFAULT_DATA_PANEL_LAYOUT.sizes[topPanel];

  const collapsed = {
    tables: value?.collapsed?.tables === true,
    views: value?.collapsed?.views === true,
  };

  if (collapsed.tables && collapsed.views) {
    collapsed.tables = false;
    collapsed.views = false;
  }

  return {
    order,
    sizes: {
      [topPanel]: topPanelSize,
      [bottomPanel]: 100 - topPanelSize,
    } as Record<DataPanelId, number>,
    collapsed,
  };
}

export function loadDataPanelLayout(storage: StorageLike | null = getBrowserStorage()): DataPanelLayout {
  if (!storage) {
    return DEFAULT_DATA_PANEL_LAYOUT;
  }

  const raw = storage.getItem(DATA_PANELS_LAYOUT_KEY);
  if (!raw) {
    return DEFAULT_DATA_PANEL_LAYOUT;
  }

  try {
    return normalizeDataPanelLayout(JSON.parse(raw) as Partial<DataPanelLayout>);
  } catch {
    return DEFAULT_DATA_PANEL_LAYOUT;
  }
}

export function persistDataPanelLayout(
  layout: DataPanelLayout,
  storage: StorageLike | null = getBrowserStorage(),
) {
  if (!storage) {
    return;
  }

  try {
    storage.setItem(DATA_PANELS_LAYOUT_KEY, JSON.stringify(normalizeDataPanelLayout(layout)));
  } catch {
    // Ignore storage failures and fall back to in-memory state.
  }
}

export function setDataPanelTopSize(layout: DataPanelLayout, topSize: number): DataPanelLayout {
  const normalized = normalizeDataPanelLayout(layout);
  const [topPanel, bottomPanel] = normalized.order;
  const clampedTopSize = clampDataPanelSize(topSize);

  return {
    ...normalized,
    sizes: {
      [topPanel]: clampedTopSize,
      [bottomPanel]: 100 - clampedTopSize,
    } as Record<DataPanelId, number>,
  };
}

export function setDataPanelCollapsed(
  layout: DataPanelLayout,
  panelId: DataPanelId,
  collapsed: boolean,
): DataPanelLayout {
  const normalized = normalizeDataPanelLayout(layout);
  const otherPanel = DATA_PANEL_IDS.find((candidate) => candidate !== panelId) ?? 'tables';
  if (collapsed && normalized.collapsed[otherPanel]) {
    return normalized;
  }

  return {
    ...normalized,
    collapsed: {
      ...normalized.collapsed,
      [panelId]: collapsed,
    },
  };
}

export function toggleDataPanelCollapsed(
  layout: DataPanelLayout,
  panelId: DataPanelId,
): DataPanelLayout {
  return setDataPanelCollapsed(layout, panelId, !layout.collapsed[panelId]);
}

export function swapDataPanels(layout: DataPanelLayout): DataPanelLayout {
  return normalizeDataPanelLayout({
    ...layout,
    order: [layout.order[1], layout.order[0]],
  });
}

export function createDataPanelLayoutStore(storage: StorageLike | null = getBrowserStorage()) {
  const store = writable<DataPanelLayout>(loadDataPanelLayout(storage));

  function commit(next: DataPanelLayout) {
    const normalized = normalizeDataPanelLayout(next);
    persistDataPanelLayout(normalized, storage);
    store.set(normalized);
  }

  return {
    subscribe: store.subscribe,
    reset() {
      commit(DEFAULT_DATA_PANEL_LAYOUT);
    },
    set(next: DataPanelLayout) {
      commit(next);
    },
    setTopSize(topSize: number) {
      commit(setDataPanelTopSize(get(store), topSize));
    },
    setCollapsed(panelId: DataPanelId, collapsed: boolean) {
      commit(setDataPanelCollapsed(get(store), panelId, collapsed));
    },
    toggleCollapsed(panelId: DataPanelId) {
      commit(toggleDataPanelCollapsed(get(store), panelId));
    },
    swapOrder() {
      commit(swapDataPanels(get(store)));
    },
  };
}

export type GridRefreshInterval = (typeof GRID_REFRESH_INTERVALS)[number];

export interface GridRefreshSnapshot {
  surfaceKey: string | null;
  intervalMs: GridRefreshInterval;
  lastRefreshedAt: number | null;
  inFlight: boolean;
}

export const DEFAULT_GRID_REFRESH_INTERVAL: GridRefreshInterval = 0;

function isGridRefreshInterval(value: unknown): value is GridRefreshInterval {
  return typeof value === 'number' && GRID_REFRESH_INTERVALS.includes(value as GridRefreshInterval);
}

export function normalizeGridRefreshInterval(value: unknown): GridRefreshInterval {
  return isGridRefreshInterval(value) ? value : DEFAULT_GRID_REFRESH_INTERVAL;
}

export function buildGridRefreshStorageKey(surfaceKey: string): string {
  return `${GRID_REFRESH_KEY_PREFIX}${surfaceKey}`;
}

export function loadGridRefreshInterval(
  surfaceKey: string,
  storage: StorageLike | null = getBrowserStorage(),
): GridRefreshInterval {
  if (!storage) {
    return DEFAULT_GRID_REFRESH_INTERVAL;
  }

  const raw = storage.getItem(buildGridRefreshStorageKey(surfaceKey));
  if (!raw) {
    return DEFAULT_GRID_REFRESH_INTERVAL;
  }

  try {
    return normalizeGridRefreshInterval(JSON.parse(raw));
  } catch {
    return DEFAULT_GRID_REFRESH_INTERVAL;
  }
}

export function persistGridRefreshInterval(
  surfaceKey: string,
  intervalMs: GridRefreshInterval,
  storage: StorageLike | null = getBrowserStorage(),
) {
  if (!storage) {
    return;
  }

  try {
    storage.setItem(
      buildGridRefreshStorageKey(surfaceKey),
      JSON.stringify(normalizeGridRefreshInterval(intervalMs)),
    );
  } catch {
    // Ignore storage failures and fall back to in-memory state.
  }
}

interface GridRefreshTimers {
  setInterval: typeof setInterval;
  clearInterval: typeof clearInterval;
}

interface GridRefreshControllerOptions {
  storage?: StorageLike | null;
  now?: () => number;
  timers?: GridRefreshTimers;
}

export function createGridRefreshController(
  refreshSurface: (surfaceKey: string) => Promise<void>,
  options: GridRefreshControllerOptions = {},
) {
  const storage = options.storage ?? getBrowserStorage();
  const now = options.now ?? Date.now;
  const timers = options.timers ?? {
    setInterval,
    clearInterval,
  };

  const store = writable<GridRefreshSnapshot>({
    surfaceKey: null,
    intervalMs: DEFAULT_GRID_REFRESH_INTERVAL,
    lastRefreshedAt: null,
    inFlight: false,
  });

  let timerId: ReturnType<typeof setInterval> | null = null;
  let refreshToken = 0;

  function stopTimer() {
    if (timerId !== null) {
      timers.clearInterval(timerId);
      timerId = null;
    }
  }

  function syncTimer(snapshot = get(store)) {
    stopTimer();

    if (!snapshot.surfaceKey || snapshot.intervalMs === DEFAULT_GRID_REFRESH_INTERVAL) {
      return;
    }

    timerId = timers.setInterval(() => {
      void refreshNow().catch(() => {
        // Polling failures should not stop future refreshes.
      });
    }, snapshot.intervalMs);
  }

  async function refreshNow(): Promise<boolean> {
    const snapshot = get(store);
    if (!snapshot.surfaceKey || snapshot.inFlight) {
      return false;
    }

    const token = ++refreshToken;
    store.set({
      ...snapshot,
      inFlight: true,
    });

    try {
      await refreshSurface(snapshot.surfaceKey);
      store.update((current) => {
        if (token !== refreshToken || current.surfaceKey !== snapshot.surfaceKey) {
          return current;
        }

        return {
          ...current,
          inFlight: false,
          lastRefreshedAt: now(),
        };
      });
      return true;
    } catch (error) {
      store.update((current) => {
        if (token !== refreshToken || current.surfaceKey !== snapshot.surfaceKey) {
          return current;
        }

        return {
          ...current,
          inFlight: false,
        };
      });
      throw error;
    }
  }

  return {
    subscribe: store.subscribe,
    setSurface(surfaceKey: string | null) {
      refreshToken += 1;
      store.set({
        surfaceKey,
        intervalMs: surfaceKey
          ? loadGridRefreshInterval(surfaceKey, storage)
          : DEFAULT_GRID_REFRESH_INTERVAL,
        lastRefreshedAt: null,
        inFlight: false,
      });
      syncTimer();
    },
    setIntervalMs(intervalMs: GridRefreshInterval) {
      store.update((current) => {
        const normalizedInterval = normalizeGridRefreshInterval(intervalMs);
        if (current.surfaceKey) {
          persistGridRefreshInterval(current.surfaceKey, normalizedInterval, storage);
        }

        return {
          ...current,
          intervalMs: normalizedInterval,
        };
      });
      syncTimer();
    },
    markRefreshed(timestamp = now()) {
      store.update((current) => ({
        ...current,
        lastRefreshedAt: timestamp,
      }));
    },
    refreshNow,
    stop() {
      stopTimer();
    },
    destroy() {
      refreshToken += 1;
      stopTimer();
    },
  };
}

const NUMERIC_DATA_TYPES = new Set([
  'smallint',
  'integer',
  'bigint',
  'real',
  'double precision',
  'numeric',
]);

const DISTINCT_COUNT_EXCLUDED_TYPES = new Set([
  'boolean',
]);

export interface NumericQuickStat {
  columnName: string;
  label: string;
  min: number;
  max: number;
  avg: number;
  sampleCount: number;
}

export interface DistinctTextQuickStat {
  columnName: string;
  label: string;
  distinctCount: number;
  sampleCount: number;
}

export interface QuickStatsSnapshot {
  totalRows: number;
  pageRowCount: number;
  numericColumns: NumericQuickStat[];
  focusedTextColumn: DistinctTextQuickStat | null;
}

function isNumericColumn(column: ColumnInfo): boolean {
  return NUMERIC_DATA_TYPES.has(column.data_type);
}

function isDistinctEligibleColumn(column: ColumnInfo): boolean {
  return !isNumericColumn(column) && !DISTINCT_COUNT_EXCLUDED_TYPES.has(column.data_type);
}

function toFiniteNumber(value: unknown): number | null {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? value : null;
  }

  if (typeof value === 'string' && value.trim().length > 0) {
    const numericValue = Number(value);
    return Number.isFinite(numericValue) ? numericValue : null;
  }

  return null;
}

export function buildQuickStatsSnapshot({
  totalRows,
  rows,
  visibleColumns,
  focusedTextColumnName = null,
}: {
  totalRows: number;
  rows: Record<string, unknown>[];
  visibleColumns: ColumnInfo[];
  focusedTextColumnName?: string | null;
}): QuickStatsSnapshot {
  const numericColumns = visibleColumns.flatMap((column) => {
    if (!isNumericColumn(column)) {
      return [];
    }

    const values = rows
      .map((row) => toFiniteNumber(row[column.name]))
      .filter((value): value is number => value !== null);

    if (values.length === 0) {
      return [];
    }

    const sum = values.reduce((runningTotal, value) => runningTotal + value, 0);

    return [
      {
        columnName: column.name,
        label: getColumnDisplayName(column),
        min: Math.min(...values),
        max: Math.max(...values),
        avg: sum / values.length,
        sampleCount: values.length,
      },
    ];
  });

  let focusedTextColumn: DistinctTextQuickStat | null = null;
  if (focusedTextColumnName) {
    const column = visibleColumns.find((candidate) => candidate.name === focusedTextColumnName);
    if (column && isDistinctEligibleColumn(column)) {
      const values = rows
        .map((row) => row[column.name])
        .filter((value) => value != null && String(value).trim().length > 0)
        .map((value) => String(value));

      focusedTextColumn = {
        columnName: column.name,
        label: getColumnDisplayName(column),
        distinctCount: new Set(values).size,
        sampleCount: values.length,
      };
    }
  }

  return {
    totalRows: Math.max(0, totalRows),
    pageRowCount: rows.length,
    numericColumns,
    focusedTextColumn,
  };
}
