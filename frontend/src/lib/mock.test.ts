import { describe, it, expect } from 'vitest';
import {
  mockFetchTables,
  mockFetchColumns,
  mockFetchRows,
  mockFetchDisplayConfig,
  mockFetchTransientViewRows,
} from './mock';

describe('mockFetchTables', () => {
  it('returns an array of tables', () => {
    const tables = mockFetchTables();
    expect(tables.length).toBeGreaterThan(0);
    expect(tables[0]).toHaveProperty('name');
    expect(tables[0]).toHaveProperty('display_name');
    expect(tables[0]).toHaveProperty('row_count_estimate');
  });
});

describe('mockFetchColumns', () => {
  it('returns columns for a known table', () => {
    const columns = mockFetchColumns('public', 'users');
    expect(columns.length).toBeGreaterThan(0);
    expect(columns[0]).toHaveProperty('name');
    expect(columns[0]).toHaveProperty('data_type');
    expect(columns[0]).toHaveProperty('is_primary_key');
  });

  it('returns empty array for unknown table', () => {
    expect(mockFetchColumns('public', 'nonexistent')).toEqual([]);
  });
});

describe('mockFetchRows', () => {
  it('returns paginated results', () => {
    const result = mockFetchRows('public', 'users', { page: 1, page_size: 10 });
    expect(result.rows.length).toBeLessThanOrEqual(10);
    expect(result.page).toBe(1);
    expect(result.page_size).toBe(10);
    expect(result.total_rows).toBeGreaterThan(0);
  });

  it('respects page_size', () => {
    const result = mockFetchRows('public', 'users', { page: 1, page_size: 5 });
    expect(result.rows.length).toBeLessThanOrEqual(5);
  });

  it('returns consistent total_rows matching row_count_estimate', () => {
    const tables = mockFetchTables();
    const usersTable = tables.find(t => t.name === 'users');
    const result = mockFetchRows('public', 'users');
    expect(result.total_rows).toBe(usersTable?.row_count_estimate);
  });

  it('filters rows with search', () => {
    const all = mockFetchRows('public', 'users');
    const filtered = mockFetchRows('public', 'users', { search: 'Alice' });
    expect(filtered.total_rows).toBeLessThanOrEqual(all.total_rows);
  });

  it('filters rows by column filters', () => {
    const filtered = mockFetchRows('public', 'users', {
      filters: { name: 'Alice' },
    });

    expect(filtered.total_rows).toBeGreaterThan(0);
    expect(
      filtered.rows.every((row) =>
        String(row.name).toLowerCase().includes('alice'),
      ),
    ).toBe(true);
  });

  it('applies exact filters as strict equality', () => {
    const filtered = mockFetchRows('public', 'users', {
      exact_filters: { id: '1' },
    });

    expect(filtered.total_rows).toBe(1);
    expect(String(filtered.rows[0].id)).toBe('1');

    // Substring semantics would also match 10, exact must not.
    const substring = mockFetchRows('public', 'users', {
      filters: { id: '1' },
    });
    expect(substring.total_rows).toBeGreaterThan(filtered.total_rows);
  });

  it('combines multiple column filters with AND logic', () => {
    const filtered = mockFetchRows('public', 'users', {
      filters: {
        name: 'Alice',
        email: 'alice.chen',
      },
    });

    expect(filtered.total_rows).toBeGreaterThan(0);
    expect(
      filtered.rows.every((row) =>
        String(row.name).toLowerCase().includes('alice') &&
        String(row.email).toLowerCase().includes('alice.chen'),
      ),
    ).toBe(true);
  });

  it('defaults page to 1 and page_size to 50', () => {
    const result = mockFetchRows('public', 'users');
    expect(result.page).toBe(1);
    expect(result.page_size).toBe(50);
  });

  it.each([50, 100, 250, 500] as const)('echoes page_size=%d in the response', (size) => {
    const result = mockFetchRows('public', 'users', { page: 1, page_size: size });
    expect(result.page_size).toBe(size);
    expect(result.rows.length).toBeLessThanOrEqual(size);
  });

  it('page_size=100 returns at most 100 rows', () => {
    const result = mockFetchRows('public', 'users', { page: 1, page_size: 100 });
    expect(result.page_size).toBe(100);
    expect(result.rows.length).toBeLessThanOrEqual(100);
  });

  it('supports multi-column sort', () => {
    const result = mockFetchRows('public', 'users', {
      page_size: 200,
      sort: 'role:asc,id:desc',
    });

    for (let i = 1; i < result.rows.length; i += 1) {
      const prev = result.rows[i - 1];
      const curr = result.rows[i];
      const prevRole = String(prev.role ?? '');
      const currRole = String(curr.role ?? '');

      if (prevRole === currRole) {
        expect(Number(prev.id)).toBeGreaterThanOrEqual(Number(curr.id));
      } else {
        expect(prevRole <= currRole).toBe(true);
      }
    }
  });
});

describe('mockFetchTransientViewRows', () => {
  it('projects base columns plus a joined related column from users', () => {
    const result = mockFetchTransientViewRows({
      base_schema: 'public',
      base_table: 'orders',
      shape: {
        columns: [
          { source_schema: 'public', source_table: 'orders', column_name: 'id' },
          { source_schema: 'public', source_table: 'orders', column_name: 'user_id' },
          { source_schema: 'public', source_table: 'users', column_name: 'name' },
        ],
      },
      page: 1,
      page_size: 5,
    });

    expect(result.columns.map((c) => c.name)).toEqual(['id', 'user_id', 'name']);
    expect(result.rows.length).toBeLessThanOrEqual(5);
    for (const row of result.rows) {
      expect(row).toHaveProperty('id');
      expect(row).toHaveProperty('user_id');
      expect(row).toHaveProperty('name');
    }
  });

  it('prefixes colliding column names with the source table', () => {
    const result = mockFetchTransientViewRows({
      base_schema: 'public',
      base_table: 'orders',
      shape: {
        columns: [
          { source_schema: 'public', source_table: 'orders', column_name: 'status' },
          { source_schema: 'public', source_table: 'users', column_name: 'status' },
        ],
      },
      page: 1,
      page_size: 5,
    });

    expect(result.columns.map((c) => c.name)).toEqual(['orders__status', 'users__status']);
  });

  it('respects page and page_size like plain rows', () => {
    const result = mockFetchTransientViewRows({
      base_schema: 'public',
      base_table: 'orders',
      shape: {
        columns: [{ source_schema: 'public', source_table: 'orders', column_name: 'id' }],
      },
      page: 2,
      page_size: 10,
    });

    expect(result.page).toBe(2);
    expect(result.page_size).toBe(10);
    expect(result.rows.length).toBeLessThanOrEqual(10);
  });
});

describe('mockFetchDisplayConfig', () => {
  it('returns branding and tables config', () => {
    const config = mockFetchDisplayConfig();
    expect(config.branding).toHaveProperty('title');
    expect(config.branding).toHaveProperty('subtitle');
    expect(Object.keys(config.tables).length).toBeGreaterThan(0);
  });

  it('includes display names for all tables', () => {
    const tables = mockFetchTables();
    const config = mockFetchDisplayConfig();
    for (const table of tables) {
      const key = `${table.schema}.${table.name}`;
      expect(config.tables[key]).toBeDefined();
      expect(config.tables[key].display_name).toBe(table.display_name);
    }
  });
});
