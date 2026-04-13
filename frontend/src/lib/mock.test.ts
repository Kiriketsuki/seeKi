import { describe, it, expect } from 'vitest';
import { mockFetchTables, mockFetchColumns, mockFetchRows, mockFetchDisplayConfig } from './mock';

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
