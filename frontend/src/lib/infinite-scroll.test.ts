import { describe, expect, it } from 'vitest';
import {
  HARD_CAP,
  SKELETON_ROW_MARKER,
  SOFT_CAP,
  appendBatch,
  computeHasMore,
  computeRowCapState,
  createInitialState,
  isSyntheticRow,
  makeInlineErrorRow,
  makeSyntheticSkeletonRows,
  resetState,
} from './infinite-scroll';

describe('computeRowCapState', () => {
  it('returns none below soft cap', () => {
    expect(computeRowCapState(0)).toBe('none');
    expect(computeRowCapState(4999)).toBe('none');
  });

  it('returns soft at soft cap boundary', () => {
    expect(computeRowCapState(SOFT_CAP)).toBe('soft');
    expect(computeRowCapState(SOFT_CAP + 1)).toBe('soft');
    expect(computeRowCapState(HARD_CAP - 1)).toBe('soft');
  });

  it('returns hard at hard cap boundary', () => {
    expect(computeRowCapState(HARD_CAP)).toBe('hard');
    expect(computeRowCapState(HARD_CAP + 100)).toBe('hard');
  });
});

describe('computeHasMore', () => {
  it('returns false when hard cap reached', () => {
    expect(computeHasMore(HARD_CAP, 50_000, 'hard')).toBe(false);
  });

  it('returns true when loaded < total and not capped', () => {
    expect(computeHasMore(100, 500, 'none')).toBe(true);
    expect(computeHasMore(5000, 6000, 'soft')).toBe(true);
  });

  it('returns false when loaded === total', () => {
    expect(computeHasMore(500, 500, 'none')).toBe(false);
  });

  it('returns false when loaded > total', () => {
    expect(computeHasMore(600, 500, 'none')).toBe(false);
  });
});

describe('makeSyntheticSkeletonRows', () => {
  it('creates the requested count of skeleton rows', () => {
    const rows = makeSyntheticSkeletonRows(3, ['id', 'name']);
    expect(rows).toHaveLength(3);
  });

  it('marks rows with SKELETON_ROW_MARKER = skeleton', () => {
    const rows = makeSyntheticSkeletonRows(2, ['id']);
    for (const row of rows) {
      expect((row as Record<string | symbol, unknown>)[SKELETON_ROW_MARKER]).toBe('skeleton');
    }
  });

  it('does not leak marker into string keys', () => {
    const rows = makeSyntheticSkeletonRows(1, ['id']);
    expect(Object.keys(rows[0])).toEqual(['id']);
  });
});

describe('makeInlineErrorRow', () => {
  it('marks row with SKELETON_ROW_MARKER = error', () => {
    const row = makeInlineErrorRow(['id', 'name']);
    expect((row as Record<string | symbol, unknown>)[SKELETON_ROW_MARKER]).toBe('error');
  });

  it('does not leak marker into string keys', () => {
    const row = makeInlineErrorRow(['id']);
    expect(Object.keys(row)).toEqual(['id']);
  });
});

describe('isSyntheticRow', () => {
  it('detects skeleton rows', () => {
    const rows = makeSyntheticSkeletonRows(1, ['id']);
    expect(isSyntheticRow(rows[0])).toBe(true);
  });

  it('detects error rows', () => {
    expect(isSyntheticRow(makeInlineErrorRow(['id']))).toBe(true);
  });

  it('returns false for real rows', () => {
    expect(isSyntheticRow({ id: 1, name: 'Alice' })).toBe(false);
    expect(isSyntheticRow({})).toBe(false);
  });
});

describe('appendBatch', () => {
  it('merges new rows after existing real rows', () => {
    const state = createInitialState();
    const result = appendBatch(state, [{ id: 1 }, { id: 2 }], 1);
    expect(result.rows).toHaveLength(2);
    expect(result.loadedCount).toBe(2);
    expect(result.lastLoadedPage).toBe(1);
  });

  it('strips synthetic rows from state before merging', () => {
    const skeleton = makeSyntheticSkeletonRows(1, ['id'])[0];
    const state = {
      rows: [{ id: 1 }, skeleton],
      loadedCount: 1,
      lastLoadedPage: 1,
      capState: 'none' as const,
    };
    const result = appendBatch(state, [{ id: 2 }], 2);
    expect(result.rows).toHaveLength(2);
    expect(result.rows.every((r) => !isSyntheticRow(r))).toBe(true);
  });

  it('updates capState to soft when loadedCount reaches SOFT_CAP', () => {
    const state = createInitialState();
    const batch = Array.from({ length: SOFT_CAP }, (_, i) => ({ id: i }));
    const result = appendBatch(state, batch, 1);
    expect(result.capState).toBe('soft');
  });

  it('updates capState to hard when loadedCount reaches HARD_CAP', () => {
    const state = createInitialState();
    const batch = Array.from({ length: HARD_CAP }, (_, i) => ({ id: i }));
    const result = appendBatch(state, batch, 1);
    expect(result.capState).toBe('hard');
  });
});

describe('resetState', () => {
  it('returns empty initial state', () => {
    const state = appendBatch(createInitialState(), [{ id: 1 }], 1);
    const reset = resetState();
    expect(reset.rows).toHaveLength(0);
    expect(reset.loadedCount).toBe(0);
    expect(reset.lastLoadedPage).toBe(0);
    expect(reset.capState).toBe('none');
    // ensure prior state is not mutated
    expect(state.rows).toHaveLength(1);
  });
});
