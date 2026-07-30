import { describe, expect, it } from 'vitest';
import { buildJumpFilters, buildReferenceParams } from './related-rows';
import type { IncomingRelationship, ReferenceEntry, TableRelationships } from './types';

function incomingEdge(
  constraint: string,
  columns: string[],
  sourceTable: string,
  sourceColumns: string[],
): IncomingRelationship {
  return {
    constraint,
    columns,
    source: {
      schema: 'public',
      table: sourceTable,
      display_name: sourceTable,
      columns: sourceColumns,
      allowed: true,
    },
  };
}

function relationships(incoming: IncomingRelationship[]): TableRelationships {
  return { outgoing: [], incoming };
}

function referenceEntry(
  table: string,
  columns: string[],
  overrides: Partial<ReferenceEntry> = {},
): ReferenceEntry {
  return {
    schema: 'public',
    table,
    display_name: table,
    columns,
    count: 3,
    capped: false,
    ...overrides,
  };
}

describe('buildReferenceParams', () => {
  it('returns an empty object for null relationships', () => {
    expect(buildReferenceParams(null, { id: 1 })).toEqual({});
  });

  it('collects one referenced column from a single incoming edge', () => {
    const rels = relationships([incomingEdge('orders_user_fkey', ['id'], 'orders', ['user_id'])]);
    expect(buildReferenceParams(rels, { id: 42 })).toEqual({ id: '42' });
  });

  it('merges referenced columns across several incoming edges', () => {
    const rels = relationships([
      incomingEdge('orders_user_fkey', ['id'], 'orders', ['user_id']),
      incomingEdge('tickets_user_fkey', ['id'], 'tickets', ['user_id']),
    ]);
    expect(buildReferenceParams(rels, { id: 7 })).toEqual({ id: '7' });
  });

  it('omits a referenced column whose value on the row is null', () => {
    const rels = relationships([
      incomingEdge('legs_route_fkey', ['region', 'code'], 'legs', ['route_region', 'route_code']),
    ]);
    expect(buildReferenceParams(rels, { region: 'EU', code: null })).toEqual({ region: 'EU' });
  });

  it('omits a referenced column that is absent from the row', () => {
    const rels = relationships([incomingEdge('orders_user_fkey', ['id'], 'orders', ['user_id'])]);
    expect(buildReferenceParams(rels, {})).toEqual({});
  });
});

describe('buildJumpFilters', () => {
  it('returns null for null relationships', () => {
    expect(buildJumpFilters(null, { id: 1 }, referenceEntry('orders', ['user_id']))).toBeNull();
  });

  it('zips the source columns with the row\'s referenced values in order', () => {
    const rels = relationships([
      incomingEdge('legs_route_fkey', ['region', 'code'], 'legs', ['route_region', 'route_code']),
    ]);
    const entry = referenceEntry('legs', ['route_region', 'route_code']);
    expect(buildJumpFilters(rels, { region: 'EU', code: '7' }, entry)).toEqual({
      route_region: 'EU',
      route_code: '7',
    });
  });

  it('returns null when no incoming edge matches the entry', () => {
    const rels = relationships([incomingEdge('orders_user_fkey', ['id'], 'orders', ['user_id'])]);
    const entry = referenceEntry('tickets', ['user_id']);
    expect(buildJumpFilters(rels, { id: 1 }, entry)).toBeNull();
  });

  it('returns null when the row carries a null referenced value', () => {
    const rels = relationships([incomingEdge('orders_user_fkey', ['id'], 'orders', ['user_id'])]);
    const entry = referenceEntry('orders', ['user_id']);
    expect(buildJumpFilters(rels, { id: null }, entry)).toBeNull();
  });
});
