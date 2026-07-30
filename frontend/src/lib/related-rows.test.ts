import { describe, expect, it } from 'vitest';
import {
  buildJumpFilters,
  buildReferenceParams,
  hasRelatedRows,
  referenceEntryKey,
  referenceEntryLabel,
} from './related-rows';
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

  it('omits edges whose source table is outside the allowlist', () => {
    const blocked = incomingEdge('audit_user_fkey', ['id'], 'audit_log', ['user_id']);
    blocked.source.allowed = false;
    expect(buildReferenceParams(relationships([blocked]), { id: 42 })).toEqual({});
  });
});

describe('hasRelatedRows', () => {
  it('is false for null relationships and for no incoming edges', () => {
    expect(hasRelatedRows(null)).toBe(false);
    expect(hasRelatedRows(relationships([]))).toBe(false);
  });

  it('is false when every incoming edge is outside the allowlist', () => {
    const blocked = incomingEdge('audit_user_fkey', ['id'], 'audit_log', ['user_id']);
    blocked.source.allowed = false;
    expect(hasRelatedRows(relationships([blocked]))).toBe(false);
  });

  it('is true when at least one incoming edge is allowed', () => {
    const blocked = incomingEdge('audit_user_fkey', ['id'], 'audit_log', ['user_id']);
    blocked.source.allowed = false;
    const allowed = incomingEdge('orders_user_fkey', ['id'], 'orders', ['user_id']);
    expect(hasRelatedRows(relationships([blocked, allowed]))).toBe(true);
  });
});

describe('referenceEntryKey', () => {
  it('separates two edges from the same source table', () => {
    const sender = referenceEntry('messages', ['sender_id']);
    const recipient = referenceEntry('messages', ['recipient_id']);
    expect(referenceEntryKey(sender)).not.toBe(referenceEntryKey(recipient));
  });

  it('is stable for the same entry', () => {
    const entry = referenceEntry('legs', ['route_region', 'route_code']);
    expect(referenceEntryKey(entry)).toBe(referenceEntryKey(referenceEntry('legs', [
      'route_region',
      'route_code',
    ])));
  });
});

describe('referenceEntryLabel', () => {
  it('shows the display name alone when the source table appears once', () => {
    const entry = referenceEntry('orders', ['user_id'], { display_name: 'Orders' });
    expect(referenceEntryLabel(entry, [entry])).toBe('Orders');
  });

  it('appends the FK columns when the source table appears twice', () => {
    const sender = referenceEntry('messages', ['sender_id'], { display_name: 'Messages' });
    const recipient = referenceEntry('messages', ['recipient_id'], { display_name: 'Messages' });
    const entries = [sender, recipient];
    expect(referenceEntryLabel(sender, entries)).toBe('Messages (sender_id)');
    expect(referenceEntryLabel(recipient, entries)).toBe('Messages (recipient_id)');
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
