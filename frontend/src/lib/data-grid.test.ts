import { describe, it, expect } from 'vitest';
import {
  formatCellValue,
  columnWidth,
  cycleSort,
  sortStateToConfig,
  getColumnDisplayName,
  buildSortableColumn,
} from './data-grid';
import type { ColumnInfo } from './types';

function col(overrides: Partial<ColumnInfo> = {}): ColumnInfo {
  return {
    name: 'test_col',
    display_name: 'Test Col',
    data_type: 'text',
    display_type: 'Text',
    is_nullable: true,
    is_primary_key: false,
    ...overrides,
  };
}

describe('formatCellValue', () => {
  it('returns null kind for null value', () => {
    const result = formatCellValue(col(), null);
    expect(result.kind).toBe('null');
    expect(result.display).toBe('NULL');
  });

  it('returns null kind for undefined value', () => {
    const result = formatCellValue(col(), undefined);
    expect(result.kind).toBe('null');
    expect(result.display).toBe('NULL');
  });

  describe('boolean', () => {
    const boolCol = col({ data_type: 'boolean' });

    it('formats true as Yes', () => {
      const result = formatCellValue(boolCol, true);
      expect(result.kind).toBe('boolean');
      expect(result.display).toBe('Yes');
      expect(result.booleanValue).toBe(true);
    });

    it('formats false as No', () => {
      const result = formatCellValue(boolCol, false);
      expect(result.kind).toBe('boolean');
      expect(result.display).toBe('No');
      expect(result.booleanValue).toBe(false);
    });

    it('formats string "true" as Yes', () => {
      const result = formatCellValue(boolCol, 'true');
      expect(result.display).toBe('Yes');
      expect(result.booleanValue).toBe(true);
    });

    it('formats string "t" as Yes', () => {
      const result = formatCellValue(boolCol, 't');
      expect(result.display).toBe('Yes');
      expect(result.booleanValue).toBe(true);
    });

    it('formats string "false" as No', () => {
      const result = formatCellValue(boolCol, 'false');
      expect(result.display).toBe('No');
      expect(result.booleanValue).toBe(false);
    });
  });

  describe('date-only', () => {
    const dateCol = col({ data_type: 'date' });

    it('formats a valid date string', () => {
      const result = formatCellValue(dateCol, '2024-06-15');
      expect(result.kind).toBe('timestamp');
      expect(result.tooltip).toBe('2024-06-15');
      // Display varies by locale but should contain "2024" and "15"
      expect(result.display).toContain('15');
      expect(result.display).toContain('2024');
    });

    it('falls through on invalid date', () => {
      const result = formatCellValue(dateCol, 'not-a-date');
      expect(result.kind).toBe('text');
      expect(result.display).toBe('not-a-date');
    });

    it('formats a valid date string using YYYY-MM-DD preference', () => {
      const result = formatCellValue(dateCol, '2024-06-15', 'YYYY-MM-DD');
      expect(result.display).toBe('2024-06-15');
    });

    it('formats a valid date string using DD/MM/YYYY preference', () => {
      const result = formatCellValue(dateCol, '2024-06-15', 'DD/MM/YYYY');
      expect(result.display).toBe('15/06/2024');
    });
  });

  describe('datetime', () => {
    const tsCol = col({ data_type: 'timestamp without time zone' });

    it('formats space-separated datetime (Safari fix: space replaced with T)', () => {
      const result = formatCellValue(tsCol, '2024-01-15 14:30:00');
      expect(result.kind).toBe('timestamp');
      expect(result.tooltip).toBe('2024-01-15 14:30:00');
      expect(result.display).toContain('2024');
    });

    it('formats ISO datetime', () => {
      const result = formatCellValue(tsCol, '2024-01-15T14:30:00');
      expect(result.kind).toBe('timestamp');
    });

    it('formats datetime using MM/DD/YYYY preference', () => {
      const result = formatCellValue(tsCol, '2024-01-15T14:30:00', 'MM/DD/YYYY');
      expect(result.display).toContain('01/15/2024');
    });

    it('formats timestamp with time zone', () => {
      const tzCol = col({ data_type: 'timestamp with time zone' });
      const result = formatCellValue(tzCol, '2024-01-15T14:30:00Z');
      expect(result.kind).toBe('timestamp');
      expect(result.display).toContain('2024');
    });

    it('falls through on invalid datetime', () => {
      const result = formatCellValue(tsCol, 'garbage');
      expect(result.kind).toBe('text');
    });

    it('handles display_type datetime override', () => {
      const customCol = col({ data_type: 'text', display_type: 'datetime' });
      const result = formatCellValue(customCol, '2024-01-15T14:30:00');
      expect(result.kind).toBe('timestamp');
    });
  });

  describe('numeric (precision-safe)', () => {
    const numericCol = col({ data_type: 'numeric' });

    it('passes through string value without Number() cast', () => {
      const bigValue = '12345678901234567890.12345';
      const result = formatCellValue(numericCol, bigValue);
      expect(result.kind).toBe('number');
      expect(result.display).toBe(bigValue);
    });
  });

  describe('number types', () => {
    it('formats integer with locale', () => {
      const intCol = col({ data_type: 'integer' });
      const result = formatCellValue(intCol, 1234567);
      expect(result.kind).toBe('number');
      // Locale-dependent, but should be a string representation
      expect(result.display).toBeTruthy();
    });

    it('formats real (float4)', () => {
      const realCol = col({ data_type: 'real' });
      const result = formatCellValue(realCol, 3.14);
      expect(result.kind).toBe('number');
    });

    it('formats double precision', () => {
      const dblCol = col({ data_type: 'double precision' });
      const result = formatCellValue(dblCol, 3.14159265358979);
      expect(result.kind).toBe('number');
    });

    it('falls through on NaN', () => {
      const intCol = col({ data_type: 'integer' });
      const result = formatCellValue(intCol, 'not-a-number');
      expect(result.kind).toBe('text');
    });

    it('falls through on Infinity', () => {
      const intCol = col({ data_type: 'integer' });
      const result = formatCellValue(intCol, Infinity);
      expect(result.kind).toBe('text');
    });

    it('handles bigint as string from backend (> 2^53)', () => {
      const bigintCol = col({ data_type: 'bigint' });
      const result = formatCellValue(bigintCol, '9007199254740993');
      expect(result.kind).toBe('number');
    });
  });

  describe('text fallback', () => {
    it('formats plain text', () => {
      const result = formatCellValue(col(), 'hello world');
      expect(result.kind).toBe('text');
      expect(result.display).toBe('hello world');
    });

    it('converts non-string values to string', () => {
      const result = formatCellValue(col(), 42);
      expect(result.kind).toBe('text');
      expect(result.display).toBe('42');
    });
  });
});

describe('columnWidth', () => {
  it('returns 92 for boolean', () => {
    expect(columnWidth(col({ data_type: 'boolean' }))).toBe(92);
  });

  it('returns 110 for integer', () => {
    expect(columnWidth(col({ data_type: 'integer' }))).toBe(110);
  });

  it('returns 132 for numeric', () => {
    expect(columnWidth(col({ data_type: 'numeric' }))).toBe(132);
  });

  it('returns 190 for timestamp', () => {
    expect(columnWidth(col({ data_type: 'timestamp with time zone' }))).toBe(190);
  });

  it('returns 280 for uuid', () => {
    expect(columnWidth(col({ data_type: 'uuid' }))).toBe(280);
  });

  it('returns 160 for unknown type', () => {
    expect(columnWidth(col({ data_type: 'custom_type' }))).toBe(160);
  });
});

describe('sortStateToConfig', () => {
  it('returns undefined when no sort active', () => {
    expect(sortStateToConfig([])).toBeUndefined();
  });

  it('returns config object when sort active', () => {
    const result = sortStateToConfig([{ column: 'name', direction: 'asc' }]);
    expect(result).toEqual({ name: 'asc' });
  });

  it('returns config object for multi-sort state', () => {
    const result = sortStateToConfig([
      { column: 'vehicle_id', direction: 'asc' },
      { column: 'id', direction: 'desc' },
    ]);
    expect(result).toEqual({ vehicle_id: 'asc', id: 'desc' });
  });
});

describe('cycleSort', () => {
  it('prepends a new column as ascending (newest = highest priority)', () => {
    expect(cycleSort([], 'name')).toEqual([
      { column: 'name', direction: 'asc' },
    ]);
  });

  it('promotes ascending sort to descending and moves it to the front', () => {
    expect(
      cycleSort(
        [
          { column: 'vehicle_id', direction: 'asc' },
          { column: 'id', direction: 'asc' },
          { column: 'logged_at', direction: 'desc' },
        ],
        'id',
      ),
    ).toEqual([
      { column: 'id', direction: 'desc' },
      { column: 'vehicle_id', direction: 'asc' },
      { column: 'logged_at', direction: 'desc' },
    ]);
  });

  it('removes descending sort entries', () => {
    expect(
      cycleSort(
        [
          { column: 'vehicle_id', direction: 'asc' },
          { column: 'id', direction: 'desc' },
        ],
        'id',
      ),
    ).toEqual([{ column: 'vehicle_id', direction: 'asc' }]);
  });

  it('preserves other entry order when re-sorting', () => {
    expect(
      cycleSort(
        [
          { column: 'a', direction: 'asc' },
          { column: 'b', direction: 'asc' },
          { column: 'c', direction: 'desc' },
        ],
        'b',
      ),
    ).toEqual([
      { column: 'b', direction: 'desc' },
      { column: 'a', direction: 'asc' },
      { column: 'c', direction: 'desc' },
    ]);
  });
});

describe('getColumnDisplayName', () => {
  it('returns display_name when set', () => {
    expect(getColumnDisplayName(col({ display_name: 'Full Name' }))).toBe('Full Name');
  });

  it('falls back to name when display_name is empty', () => {
    expect(getColumnDisplayName(col({ name: 'user_id', display_name: '' }))).toBe('user_id');
  });
});

describe('buildSortableColumn', () => {
  it('builds a column with correct prop and name', () => {
    const c = col({ name: 'email', display_name: 'Email Address', data_type: 'text' });
    const result = buildSortableColumn(c);
    expect(result.prop).toBe('email');
    expect(result.name).toBe('Email Address');
    expect(result.sortable).toBe(true);
  });

  it('applies overrides', () => {
    const c = col({ name: 'id', data_type: 'integer' });
    const result = buildSortableColumn(c, { sortable: false });
    expect(result.sortable).toBe(false);
  });
});
