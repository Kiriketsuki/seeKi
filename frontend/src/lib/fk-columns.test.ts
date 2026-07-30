import { describe, expect, it } from 'vitest';
import { buildFkColumnMap, fkBadgeTarget, fkCellValues, hasHoppableEdge } from './fk-columns';
import type { OutgoingRelationship, TableRelationships } from './types';

function edge(
  constraint: string,
  columns: string[],
  targetTable: string,
  allowed = true,
  targetColumns = ['id'],
): OutgoingRelationship {
  return {
    constraint,
    columns,
    target: {
      schema: 'public',
      table: targetTable,
      display_name: targetTable,
      columns: targetColumns,
      allowed,
    },
  };
}

function relationships(outgoing: OutgoingRelationship[]): TableRelationships {
  return { outgoing, incoming: [] };
}

describe('buildFkColumnMap', () => {
  it('returns an empty map for null relationships', () => {
    expect(buildFkColumnMap(null).size).toBe(0);
  });

  it('maps a single-column FK to its edge', () => {
    const map = buildFkColumnMap(
      relationships([edge('orders_user_fkey', ['user_id'], 'users')]),
    );
    expect(map.get('user_id')?.[0].constraint).toBe('orders_user_fkey');
    expect(map.has('id')).toBe(false);
  });

  it('maps every member of a composite FK to the same edge', () => {
    const map = buildFkColumnMap(
      relationships([
        edge('legs_route_fkey', ['route_region', 'route_code'], 'routes', true, [
          'region',
          'code',
        ]),
      ]),
    );
    expect(map.get('route_region')?.[0].constraint).toBe('legs_route_fkey');
    expect(map.get('route_code')?.[0].constraint).toBe('legs_route_fkey');
  });

  it('collects several constraints on one column', () => {
    const map = buildFkColumnMap(
      relationships([
        edge('a_fkey', ['ref_id'], 'alpha'),
        edge('b_fkey', ['ref_id'], 'beta'),
      ]),
    );
    expect(map.get('ref_id')).toHaveLength(2);
  });
});

describe('hasHoppableEdge', () => {
  it('is false for undefined and for non-allowed targets only', () => {
    expect(hasHoppableEdge(undefined)).toBe(false);
    expect(hasHoppableEdge([edge('x', ['a'], 'hidden', false)])).toBe(false);
  });

  it('is true when any target is allowed', () => {
    expect(
      hasHoppableEdge([
        edge('x', ['a'], 'hidden', false),
        edge('y', ['a'], 'visible', true),
      ]),
    ).toBe(true);
  });
});

describe('fkBadgeTarget', () => {
  it('returns null with no edges', () => {
    expect(fkBadgeTarget(undefined)).toBeNull();
    expect(fkBadgeTarget([])).toBeNull();
  });

  it('prefers the first allowed target', () => {
    const picked = fkBadgeTarget([
      edge('x', ['a'], 'hidden', false),
      edge('y', ['a'], 'visible', true),
    ]);
    expect(picked?.target.table).toBe('visible');
  });

  it('falls back to the first edge when nothing is allowed', () => {
    const picked = fkBadgeTarget([edge('x', ['a'], 'hidden', false)]);
    expect(picked?.target.table).toBe('hidden');
  });
});

describe('fkCellValues', () => {
  it('reads a single-column FK value as a string', () => {
    const values = fkCellValues(edge('orders_user_fkey', ['user_id'], 'users'), {
      user_id: 42,
    });
    expect(values).toEqual({ user_id: '42' });
  });

  it('reads every member of a composite FK', () => {
    const composite = edge(
      'legs_route_fkey',
      ['route_region', 'route_code'],
      'routes',
      true,
      ['region', 'code'],
    );
    const values = fkCellValues(composite, {
      route_region: 'EU',
      route_code: 7,
    });
    expect(values).toEqual({ route_region: 'EU', route_code: '7' });
  });

  it('returns null when a composite member is null', () => {
    const composite = edge(
      'legs_route_fkey',
      ['route_region', 'route_code'],
      'routes',
      true,
      ['region', 'code'],
    );
    const values = fkCellValues(composite, {
      route_region: 'EU',
      route_code: null,
    });
    expect(values).toBeNull();
  });

  it('returns null when a composite member is undefined', () => {
    const composite = edge(
      'legs_route_fkey',
      ['route_region', 'route_code'],
      'routes',
      true,
      ['region', 'code'],
    );
    const values = fkCellValues(composite, { route_region: 'EU' });
    expect(values).toBeNull();
  });
});
