import { describe, it, expect, afterEach } from 'vitest';
import {
  formatCellValue,
  columnWidth,
  cycleSort,
  replaceSort,
  sortStateToConfig,
  getColumnDisplayName,
  buildSortableColumn,
  setDisplayTimeZone,
  getDisplayTimeZone,
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

  describe('json/jsonb objects', () => {
    const jsonCol = col({ data_type: 'jsonb', display_type: 'JSON' });

    it('renders an object as JSON text, not [object Object]', () => {
      const result = formatCellValue(jsonCol, { type: 'Polygon', srid: 4326 });
      expect(result.kind).toBe('text');
      expect(result.display).toBe('{"type":"Polygon","srid":4326}');
    });

    it('renders an array as JSON text', () => {
      const result = formatCellValue(jsonCol, [1, 2, 3]);
      expect(result.display).toBe('[1,2,3]');
    });
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

  describe('display timezone', () => {
    const tzCol = col({ data_type: 'timestamp with time zone' });
    const naiveCol = col({ data_type: 'timestamp without time zone' });

    // The 'YYYY-MM-DD' branch renders "<date> <time>" where <time> still comes
    // from Intl with the ambient locale (so it may be "14:30" or "2:30 PM").
    // Build the expectation the same way to assert on the zone arithmetic
    // without pinning the test machine's locale.
    function expected(date: string, hour: number, minute: number): string {
      const time = new Intl.DateTimeFormat(undefined, {
        hour: 'numeric',
        minute: '2-digit',
      }).format(new Date(2024, 0, 15, hour, minute));
      return `${date} ${time}`;
    }

    afterEach(() => {
      setDisplayTimeZone(undefined);
    });

    it('renders timestamptz in the configured zone, not the browser zone', () => {
      setDisplayTimeZone('Asia/Singapore');
      // 06:30Z is 14:30 +08. The grid must agree with the offset the backend
      // serialized rather than with wherever the viewer happens to be.
      const result = formatCellValue(tzCol, '2024-01-15T06:30:00Z', 'YYYY-MM-DD');
      expect(result.display).toBe(expected('2024-01-15', 14, 30));
    });

    it('keeps the calendar date of the configured zone across a day boundary', () => {
      setDisplayTimeZone('Asia/Singapore');
      // 23:00Z on the 15th is 07:00 +08 on the 16th. Reading parts off the Date
      // in the browser's zone would name the wrong day here.
      const result = formatCellValue(tzCol, '2024-01-15T23:00:00Z', 'YYYY-MM-DD');
      expect(result.display).toBe(expected('2024-01-16', 7, 0));
    });

    it('shows offset-less timestamps verbatim rather than shifting them', () => {
      setDisplayTimeZone('Asia/Singapore');
      // A naive value is already wall-clock in the display zone, so it must not
      // be converted — 14:30 stays 14:30 whatever the browser zone is.
      const result = formatCellValue(naiveCol, '2024-01-15 14:30:00', 'YYYY-MM-DD');
      expect(result.display).toBe(expected('2024-01-15', 14, 30));
    });

    it('respects an explicit +08:00 offset on the wire value', () => {
      setDisplayTimeZone('Asia/Singapore');
      const result = formatCellValue(tzCol, '2024-01-15T14:30:00+08:00', 'YYYY-MM-DD');
      expect(result.display).toBe(expected('2024-01-15', 14, 30));
    });

    it('preserves the raw wire value in the tooltip', () => {
      setDisplayTimeZone('Asia/Singapore');
      const raw = '2024-01-15T14:30:00+08:00';
      expect(formatCellValue(tzCol, raw).tooltip).toBe(raw);
    });

    it('does not read a bare date\'s day as an offset', () => {
      setDisplayTimeZone('Asia/Singapore');
      // "2024-01-15" has no time component; treating the trailing "-15" as a
      // -15:00 offset would shift the rendered day.
      const result = formatCellValue(naiveCol, '2024-01-15', 'YYYY-MM-DD');
      expect(result.display).toContain('2024-01-15');
    });

    it('ignores an unrecognised zone instead of throwing', () => {
      setDisplayTimeZone('Mars/Olympus');
      expect(getDisplayTimeZone()).toBeUndefined();
      expect(formatCellValue(tzCol, '2024-01-15T06:30:00Z').kind).toBe('timestamp');
    });

    it('applies a changed zone to subsequent formatting', () => {
      setDisplayTimeZone('Asia/Singapore');
      const sgt = formatCellValue(tzCol, '2024-01-15T06:30:00Z', 'YYYY-MM-DD');
      setDisplayTimeZone('UTC');
      const utc = formatCellValue(tzCol, '2024-01-15T06:30:00Z', 'YYYY-MM-DD');
      expect(sgt.display).toBe(expected('2024-01-15', 14, 30));
      expect(utc.display).toBe(expected('2024-01-15', 6, 30));
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

describe('replaceSort', () => {
  it('returns a single-column asc sort when column is not yet sorted', () => {
    expect(replaceSort([], 'name')).toEqual([{ column: 'name', direction: 'asc' }]);
  });

  it('replaces any existing multi-sort with a new single-column asc sort', () => {
    expect(
      replaceSort(
        [
          { column: 'a', direction: 'asc' },
          { column: 'b', direction: 'desc' },
        ],
        'c',
      ),
    ).toEqual([{ column: 'c', direction: 'asc' }]);
  });

  it('cycles the existing ascending sort to descending and drops other columns', () => {
    expect(
      replaceSort(
        [
          { column: 'a', direction: 'asc' },
          { column: 'b', direction: 'asc' },
        ],
        'a',
      ),
    ).toEqual([{ column: 'a', direction: 'desc' }]);
  });

  it('returns an empty sort when the target column was descending (full cycle)', () => {
    expect(
      replaceSort(
        [
          { column: 'a', direction: 'desc' },
          { column: 'b', direction: 'asc' },
        ],
        'a',
      ),
    ).toEqual([]);
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
