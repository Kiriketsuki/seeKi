export const SOFT_CAP = 5_000;
export const HARD_CAP = 10_000;
export const SKELETON_ROW_MARKER: unique symbol = Symbol('seeki.skeleton');

type MarkedRow = { [SKELETON_ROW_MARKER]?: 'skeleton' | 'error' };

export type RowCapState = 'none' | 'soft' | 'hard';

export interface InfiniteScrollState {
  rows: Record<string, unknown>[];
  loadedCount: number;
  lastLoadedPage: number;
  capState: RowCapState;
}

export function createInitialState(): InfiniteScrollState {
  return {
    rows: [],
    loadedCount: 0,
    lastLoadedPage: 0,
    capState: 'none',
  };
}

export function computeRowCapState(loadedCount: number): RowCapState {
  if (loadedCount >= HARD_CAP) return 'hard';
  if (loadedCount >= SOFT_CAP) return 'soft';
  return 'none';
}

export function computeHasMore(
  loadedCount: number,
  totalRows: number,
  capState: RowCapState,
): boolean {
  if (capState === 'hard') return false;
  return loadedCount < totalRows;
}

export function makeSyntheticSkeletonRows(
  count: number,
  columns: string[],
): Record<string, unknown>[] {
  return Array.from({ length: count }, () => {
    const row: Record<string, unknown> & MarkedRow = Object.fromEntries(
      columns.map((c) => [c, null]),
    );
    row[SKELETON_ROW_MARKER] = 'skeleton';
    return row;
  });
}

export function makeInlineErrorRow(columns: string[]): Record<string, unknown> {
  const row: Record<string, unknown> & MarkedRow = Object.fromEntries(
    columns.map((c) => [c, null]),
  );
  row[SKELETON_ROW_MARKER] = 'error';
  return row;
}

export function isSyntheticRow(row: Record<string, unknown>): boolean {
  const marker = (row as MarkedRow)[SKELETON_ROW_MARKER];
  return marker === 'skeleton' || marker === 'error';
}

export function appendBatch(
  state: InfiniteScrollState,
  newRows: Record<string, unknown>[],
  page: number,
): InfiniteScrollState {
  const cleanRows = state.rows.filter((r) => !isSyntheticRow(r));
  const merged = [...cleanRows, ...newRows];
  const capState = computeRowCapState(merged.length);
  const rows = capState === 'hard' ? merged.slice(0, HARD_CAP) : merged;
  return {
    rows,
    loadedCount: rows.length,
    lastLoadedPage: page,
    capState,
  };
}

export function resetState(): InfiniteScrollState {
  return createInitialState();
}
