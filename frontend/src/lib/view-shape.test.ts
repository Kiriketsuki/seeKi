import { describe, it, expect, beforeEach } from 'vitest';
import {
  buildRelatedShape,
  buildSourceDefinition,
  fkSourceId,
  loadRelatedColumns,
  relatedColumnsStorageKey,
  resolveViewColumnOutputNames,
  sanitizeGeneratedIdentifier,
  saveRelatedColumns,
} from './view-shape';
import type { ColumnInfo, PickedRelatedColumn } from './types';

// The test environment runs in plain Node, which has no localStorage global.
// A minimal in-memory stand-in is enough to exercise the round-trip helpers,
// which only call getItem, setItem, removeItem, and clear.
class MemoryStorage {
  private store = new Map<string, string>();

  getItem(key: string): string | null {
    return this.store.has(key) ? this.store.get(key)! : null;
  }

  setItem(key: string, value: string): void {
    this.store.set(key, value);
  }

  removeItem(key: string): void {
    this.store.delete(key);
  }

  clear(): void {
    this.store.clear();
  }
}

if (typeof globalThis.localStorage === 'undefined') {
  (globalThis as unknown as { localStorage: MemoryStorage }).localStorage = new MemoryStorage();
}

const ORDERS_COLUMNS: ColumnInfo[] = [
  { name: 'id', display_name: 'ID', data_type: 'integer', display_type: 'number', is_nullable: false, is_primary_key: true },
  { name: 'user_id', display_name: 'User ID', data_type: 'integer', display_type: 'number', is_nullable: false, is_primary_key: false },
  { name: 'status', display_name: 'Status', data_type: 'varchar', display_type: 'text', is_nullable: false, is_primary_key: false },
];

describe('buildSourceDefinition', () => {
  it('returns null for the base table', () => {
    const source = buildSourceDefinition({
      mode: 'base',
      baseSchema: 'public',
      baseTable: 'orders',
      selectedSchema: 'public',
      selectedTable: 'orders',
      activeSourceId: null,
      selfEntityColumn: '',
      selfOrderColumn: '',
      selfDirection: 'previous',
      selectedBaseMatchColumn: '',
      selectedSourceMatchColumn: '',
    });
    expect(source).toBeNull();
  });

  it('builds an fk source ref with a deterministic id', () => {
    const source = buildSourceDefinition({
      mode: 'fk',
      baseSchema: 'public',
      baseTable: 'orders',
      selectedSchema: 'public',
      selectedTable: 'users',
      activeSourceId: null,
      selfEntityColumn: '',
      selfOrderColumn: '',
      selfDirection: 'previous',
      selectedBaseMatchColumn: '',
      selectedSourceMatchColumn: '',
    });
    expect(source).toEqual({
      id: 'fk-public.users',
      kind: 'fk',
      schema: 'public',
      table: 'users',
      label: 'public.users',
    });
  });

  it('reuses an existing source id when editing an established column', () => {
    const source = buildSourceDefinition({
      mode: 'fk',
      baseSchema: 'public',
      baseTable: 'orders',
      selectedSchema: 'public',
      selectedTable: 'users',
      activeSourceId: 'fk-existing',
      selfEntityColumn: '',
      selfOrderColumn: '',
      selfDirection: 'previous',
      selectedBaseMatchColumn: '',
      selectedSourceMatchColumn: '',
    });
    expect(source?.id).toBe('fk-existing');
  });

  it('builds a self-join source ref for the previous-row direction', () => {
    const source = buildSourceDefinition({
      mode: 'self',
      baseSchema: 'public',
      baseTable: 'orders',
      selectedSchema: 'public',
      selectedTable: 'orders',
      activeSourceId: null,
      selfEntityColumn: 'user_id',
      selfOrderColumn: 'created_at',
      selfDirection: 'previous',
      selectedBaseMatchColumn: '',
      selectedSourceMatchColumn: '',
    });
    expect(source).toEqual({
      id: 'self-previous',
      kind: 'self',
      schema: 'public',
      table: 'orders',
      label: 'This table again (previous row)',
      self: {
        entity_column: 'user_id',
        order_column: 'created_at',
        direction: 'previous',
      },
    });
  });

  it('builds a match source ref', () => {
    const source = buildSourceDefinition({
      mode: 'match',
      baseSchema: 'public',
      baseTable: 'orders',
      selectedSchema: 'public',
      selectedTable: 'warehouses',
      activeSourceId: null,
      selfEntityColumn: '',
      selfOrderColumn: '',
      selfDirection: 'previous',
      selectedBaseMatchColumn: 'warehouse_code',
      selectedSourceMatchColumn: 'code',
    });
    expect(source).toEqual({
      id: 'match-public.warehouses',
      kind: 'match',
      schema: 'public',
      table: 'warehouses',
      label: 'Match public.warehouses',
      match: {
        base_column: 'warehouse_code',
        source_column: 'code',
      },
    });
  });
});

describe('fkSourceId', () => {
  it('is deterministic for a given schema and table', () => {
    expect(fkSourceId('public', 'users')).toBe('fk-public.users');
    expect(fkSourceId('public', 'users')).toBe(fkSourceId('public', 'users'));
  });
});

describe('buildRelatedShape', () => {
  it('projects every base column plus each picked related column', () => {
    const picked: PickedRelatedColumn[] = [
      { schema: 'public', table: 'users', tableDisplayName: 'Users', column: 'name', columnDisplayName: 'Name' },
    ];
    const shape = buildRelatedShape('public', 'orders', ORDERS_COLUMNS, picked);

    expect(shape.columns).toHaveLength(4);
    expect(shape.columns.slice(0, 3)).toEqual([
      { source_schema: 'public', source_table: 'orders', column_name: 'id' },
      { source_schema: 'public', source_table: 'orders', column_name: 'user_id' },
      { source_schema: 'public', source_table: 'orders', column_name: 'status' },
    ]);
    expect(shape.columns[3]).toEqual({
      source_id: 'fk-public.users',
      source_schema: 'public',
      source_table: 'users',
      column_name: 'name',
    });
    expect(shape.sources).toEqual([
      { id: 'fk-public.users', kind: 'fk', schema: 'public', table: 'users', label: 'Users' },
    ]);
    expect(shape.filters).toEqual({});
  });

  it('deduplicates the source ref when two columns come from the same related table', () => {
    const picked: PickedRelatedColumn[] = [
      { schema: 'public', table: 'users', tableDisplayName: 'Users', column: 'name', columnDisplayName: 'Name' },
      { schema: 'public', table: 'users', tableDisplayName: 'Users', column: 'email', columnDisplayName: 'Email' },
    ];
    const shape = buildRelatedShape('public', 'orders', ORDERS_COLUMNS, picked);

    expect(shape.sources).toHaveLength(1);
    expect(shape.columns).toHaveLength(5);
  });

  it('returns only the base columns when nothing is picked', () => {
    const shape = buildRelatedShape('public', 'orders', ORDERS_COLUMNS, []);
    expect(shape.columns).toHaveLength(3);
    expect(shape.sources).toEqual([]);
  });

  it('is stable across repeated calls with the same inputs', () => {
    const picked: PickedRelatedColumn[] = [
      { schema: 'public', table: 'users', tableDisplayName: 'Users', column: 'name', columnDisplayName: 'Name' },
    ];
    const first = buildRelatedShape('public', 'orders', ORDERS_COLUMNS, picked);
    const second = buildRelatedShape('public', 'orders', ORDERS_COLUMNS, picked);
    expect(first).toEqual(second);
  });
});

describe('related columns localStorage round-trip', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('returns an empty array when nothing is stored', () => {
    expect(loadRelatedColumns('public', 'orders')).toEqual([]);
  });

  it('round-trips a saved list through localStorage', () => {
    const picked: PickedRelatedColumn[] = [
      { schema: 'public', table: 'users', tableDisplayName: 'Users', column: 'name', columnDisplayName: 'Name' },
    ];
    saveRelatedColumns('public', 'orders', picked);
    expect(loadRelatedColumns('public', 'orders')).toEqual(picked);
  });

  it('scopes storage per schema and table', () => {
    const picked: PickedRelatedColumn[] = [
      { schema: 'public', table: 'users', tableDisplayName: 'Users', column: 'name', columnDisplayName: 'Name' },
    ];
    saveRelatedColumns('public', 'orders', picked);
    expect(loadRelatedColumns('public', 'tickets')).toEqual([]);
    expect(relatedColumnsStorageKey('public', 'orders')).not.toBe(
      relatedColumnsStorageKey('public', 'tickets'),
    );
  });

  it('removes the stored key entirely when saving an empty list', () => {
    saveRelatedColumns('public', 'orders', [
      { schema: 'public', table: 'users', tableDisplayName: 'Users', column: 'name', columnDisplayName: 'Name' },
    ]);
    saveRelatedColumns('public', 'orders', []);
    expect(localStorage.getItem(relatedColumnsStorageKey('public', 'orders'))).toBeNull();
    expect(loadRelatedColumns('public', 'orders')).toEqual([]);
  });

  it('returns an empty array for malformed stored JSON', () => {
    localStorage.setItem(relatedColumnsStorageKey('public', 'orders'), '{not json');
    expect(loadRelatedColumns('public', 'orders')).toEqual([]);
  });

  it('returns an empty array when stored value is not an array', () => {
    localStorage.setItem(relatedColumnsStorageKey('public', 'orders'), JSON.stringify({ foo: 'bar' }));
    expect(loadRelatedColumns('public', 'orders')).toEqual([]);
  });
});

describe('resolveViewColumnOutputNames', () => {
  it('keeps bare names when there is no collision', () => {
    const names = resolveViewColumnOutputNames([
      { source_schema: 'public', source_table: 'orders', column_name: 'id' },
      { source_schema: 'public', source_table: 'users', column_name: 'name' },
    ]);
    expect(names).toEqual(['id', 'name']);
  });

  it('prefixes colliding column names with the source table', () => {
    const names = resolveViewColumnOutputNames([
      { source_schema: 'public', source_table: 'orders', column_name: 'status' },
      { source_schema: 'public', source_table: 'users', column_name: 'status' },
    ]);
    expect(names).toEqual(['orders__status', 'users__status']);
  });

  it('prefixes a colliding column with its sanitized source id, as the backend does', () => {
    // output_name_prefix_for_saved_column prefers source_id over source_table,
    // and sanitize_generated_identifier turns "." into "_".
    const names = resolveViewColumnOutputNames([
      { source_schema: 'public', source_table: 'orders', column_name: 'name' },
      {
        source_id: 'fk-public.users',
        source_schema: 'public',
        source_table: 'users',
        column_name: 'name',
      },
    ]);
    expect(names).toEqual(['orders__name', 'fk-public_users__name']);
  });

  it('matches the output names of a shape buildRelatedShape produces', () => {
    const shape = buildRelatedShape(
      'public',
      'orders',
      [
        { name: 'id', display_name: 'ID', data_type: 'integer', display_type: 'number', is_nullable: false, is_primary_key: true },
        { name: 'name', display_name: 'Name', data_type: 'varchar', display_type: 'text', is_nullable: true, is_primary_key: false },
      ],
      [
        {
          schema: 'public',
          table: 'users',
          tableDisplayName: 'Users',
          column: 'name',
          columnDisplayName: 'Name',
        },
      ],
    );
    expect(resolveViewColumnOutputNames(shape.columns)).toEqual([
      'id',
      'orders__name',
      'fk-public_users__name',
    ]);
  });
});

describe('sanitizeGeneratedIdentifier', () => {
  it('keeps alphanumerics, underscore, hyphen and space', () => {
    expect(sanitizeGeneratedIdentifier('fk-public users_1')).toBe('fk-public users_1');
  });

  it('replaces every other character', () => {
    expect(sanitizeGeneratedIdentifier('fk-public.users')).toBe('fk-public_users');
  });

  it('falls back to "source" when nothing readable survives', () => {
    expect(sanitizeGeneratedIdentifier('   ')).toBe('source');
    expect(sanitizeGeneratedIdentifier('')).toBe('source');
  });
});
