import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  buildGridRefreshStorageKey,
  buildQuickStatsSnapshot,
  createGridRefreshController,
  DEFAULT_DATA_PANEL_LAYOUT,
  type GridRefreshSnapshot,
  normalizeDataPanelLayout,
  setDataPanelCollapsed,
  setDataPanelTopSize,
  swapDataPanels,
} from './stores';
import type { ColumnInfo } from './types';

function createStorage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial));

  return {
    getItem(key: string) {
      return values.get(key) ?? null;
    },
    setItem(key: string, value: string) {
      values.set(key, value);
    },
  };
}

function column(overrides: Partial<ColumnInfo> = {}): ColumnInfo {
  return {
    name: 'value',
    display_name: 'Value',
    data_type: 'text',
    display_type: 'Text',
    is_nullable: true,
    is_primary_key: false,
    ...overrides,
  };
}

describe('data panel layout helpers', () => {
  it('falls back to the default 60/40 layout', () => {
    expect(normalizeDataPanelLayout(null)).toEqual(DEFAULT_DATA_PANEL_LAYOUT);
  });

  it('keeps at least one panel expanded', () => {
    const tablesCollapsed = setDataPanelCollapsed(DEFAULT_DATA_PANEL_LAYOUT, 'tables', true);

    expect(tablesCollapsed.collapsed.tables).toBe(true);
    expect(tablesCollapsed.collapsed.views).toBe(false);

    const attemptToCollapseBoth = setDataPanelCollapsed(tablesCollapsed, 'views', true);
    expect(attemptToCollapseBoth).toEqual(tablesCollapsed);
  });

  it('swaps order and resizes the top panel by percentage', () => {
    const swapped = swapDataPanels(DEFAULT_DATA_PANEL_LAYOUT);
    expect(swapped.order).toEqual(['views', 'tables']);
    expect(swapped.sizes.views).toBe(40);
    expect(swapped.sizes.tables).toBe(60);

    const resized = setDataPanelTopSize(swapped, 72);
    expect(resized.sizes.views).toBe(72);
    expect(resized.sizes.tables).toBe(28);
  });
});

describe('createGridRefreshController', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('persists auto-refresh settings per surface key', () => {
    const storage = createStorage();
    const controller = createGridRefreshController(async () => {}, { storage });
    let snapshot: GridRefreshSnapshot = {
      surfaceKey: null,
      intervalMs: 0,
      lastRefreshedAt: null,
      inFlight: false,
    };

    const unsubscribe = controller.subscribe((value) => {
      snapshot = value;
    });

    controller.setSurface('table:public.vehicle_logs');
    controller.setIntervalMs(60_000);

    expect(storage.getItem(buildGridRefreshStorageKey('table:public.vehicle_logs'))).toBe('60000');
    expect(snapshot.intervalMs).toBe(60_000);

    controller.setSurface('table:public.vehicle_logs');
    expect(snapshot.intervalMs).toBe(60_000);

    unsubscribe();
    controller.destroy();
  });

  it('skips overlapping refresh ticks while a request is in flight', async () => {
    const storage = createStorage();
    let resolveFirstRefresh = () => {};
    const firstRefresh = new Promise<void>((resolve) => {
      resolveFirstRefresh = resolve;
    });
    const refreshSurface = vi
      .fn<(_: string) => Promise<void>>()
      .mockImplementationOnce(() => firstRefresh)
      .mockResolvedValue(undefined);

    let now = 1_000;
    const controller = createGridRefreshController(refreshSurface, {
      storage,
      now: () => now,
    });
    let snapshot: GridRefreshSnapshot = {
      surfaceKey: null,
      intervalMs: 0,
      lastRefreshedAt: null,
      inFlight: false,
    };

    const unsubscribe = controller.subscribe((value) => {
      snapshot = value;
    });

    controller.setSurface('view:42');
    controller.setIntervalMs(15_000);

    await vi.advanceTimersByTimeAsync(15_000);
    expect(refreshSurface).toHaveBeenCalledTimes(1);
    expect(snapshot.inFlight).toBe(true);

    await vi.advanceTimersByTimeAsync(15_000);
    expect(refreshSurface).toHaveBeenCalledTimes(1);

    now = 2_000;
    resolveFirstRefresh();
    await Promise.resolve();
    expect(snapshot.inFlight).toBe(false);
    expect(snapshot.lastRefreshedAt).toBe(2_000);

    await vi.advanceTimersByTimeAsync(15_000);
    expect(refreshSurface).toHaveBeenCalledTimes(2);

    unsubscribe();
    controller.destroy();
  });
});

describe('buildQuickStatsSnapshot', () => {
  it('computes page-level numeric and distinct text stats', () => {
    const snapshot = buildQuickStatsSnapshot({
      totalRows: 18,
      rows: [
        {
          battery_soc: '11.5',
          vehicle_id: 'VEH-1',
        },
        {
          battery_soc: 19,
          vehicle_id: 'VEH-1',
        },
        {
          battery_soc: 30,
          vehicle_id: 'VEH-2',
        },
      ],
      visibleColumns: [
        column({
          name: 'battery_soc',
          display_name: 'Battery SOC',
          data_type: 'numeric',
        }),
        column({
          name: 'vehicle_id',
          display_name: 'Vehicle',
          data_type: 'text',
        }),
      ],
      focusedTextColumnName: 'vehicle_id',
    });

    expect(snapshot.totalRows).toBe(18);
    expect(snapshot.pageRowCount).toBe(3);
    expect(snapshot.numericColumns).toHaveLength(1);
    expect(snapshot.numericColumns[0]).toMatchObject({
      columnName: 'battery_soc',
      label: 'Battery SOC',
      min: 11.5,
      max: 30,
      sampleCount: 3,
    });
    expect(snapshot.numericColumns[0].avg).toBeCloseTo(20.1667, 3);
    expect(snapshot.focusedTextColumn).toMatchObject({
      columnName: 'vehicle_id',
      label: 'Vehicle',
      distinctCount: 2,
      sampleCount: 3,
    });
  });
});
