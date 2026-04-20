export const SOFT_CAP = 5_000;
export const HARD_CAP = 10_000;
export const SKELETON_ROW_MARKER = '__seekiRowState';

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
  return Array.from({ length: count }, (_, i) => {
    const row: Record<string, unknown> = { [SKELETON_ROW_MARKER]: 'skeleton' };
    for (const col of columns) {
      row[col] = null;
    }
    row['__skeletonIndex'] = i;
    return row;
  });
}

export function makeInlineErrorRow(columns: string[]): Record<string, unknown> {
  const row: Record<string, unknown> = { [SKELETON_ROW_MARKER]: 'error' };
  for (const col of columns) {
    row[col] = null;
  }
  return row;
}

export function isSyntheticRow(row: Record<string, unknown>): boolean {
  return row[SKELETON_ROW_MARKER] === 'skeleton' || row[SKELETON_ROW_MARKER] === 'error';
}

export function appendBatch(
  state: InfiniteScrollState,
  newRows: Record<string, unknown>[],
  page: number,
  totalRows: number,
): InfiniteScrollState {
  const cleanRows = state.rows.filter((r) => !isSyntheticRow(r));
  const merged = [...cleanRows, ...newRows];
  const loadedCount = merged.length;
  return {
    rows: merged,
    loadedCount,
    lastLoadedPage: page,
    capState: computeRowCapState(loadedCount),
  };
}

export function resetState(): InfiniteScrollState {
  return createInitialState();
}
