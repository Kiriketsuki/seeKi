import type { ColumnRegular } from '@revolist/svelte-datagrid';
import type {
  ColumnInfo,
  DateFormatPreference,
  SortState,
} from './types';

const INTEGER_TYPES = new Set([
  'smallint',
  'integer',
  'bigint',
]);

const NUMBER_TYPES = new Set([
  ...INTEGER_TYPES,
  'real',
  'double precision',
]);

// Types where the backend sends a full-precision string to avoid JS float truncation.
// Display as right-aligned numbers with tabular-nums but skip Number() casting.
const NUMERIC_TEXT_TYPES = new Set([
  'numeric',
]);

const DATE_ONLY_TYPES = new Set([
  'date',
]);

const DATETIME_TYPES = new Set([
  'timestamp',
  'timestamp without time zone',
  'timestamp with time zone',
]);

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
});

const formatter = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
  hour: 'numeric',
  minute: '2-digit',
});

const timeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: 'numeric',
  minute: '2-digit',
});

export interface FormattedCellValue {
  kind: 'null' | 'boolean' | 'number' | 'timestamp' | 'text';
  display: string;
  tooltip?: string;
  booleanValue?: boolean;
}

export function columnWidth(col: ColumnInfo): number {
  switch (col.data_type) {
    case 'boolean':
      return 92;
    case 'smallint':
    case 'integer':
      return 110;
    case 'bigint':
    case 'real':
    case 'double precision':
    case 'numeric':
      return 132;
    case 'date':
      return 132;
    case 'time without time zone':
    case 'time with time zone':
      return 110;
    case 'timestamp':
    case 'timestamp without time zone':
    case 'timestamp with time zone':
      return 190;
    case 'uuid':
      return 280;
    case 'json':
    case 'jsonb':
      return 250;
    default:
      return 160;
  }
}

export function sortStateToConfig(
  sortState: SortState,
): Record<string, SortState['direction']> | undefined {
  if (!sortState.column || !sortState.direction) {
    return undefined;
  }

  return {
    [sortState.column]: sortState.direction,
  };
}

export function getColumnDisplayName(column: ColumnInfo): string {
  return column.display_name || column.name;
}

export function formatCellValue(
  column: ColumnInfo,
  value: unknown,
  dateFormat: DateFormatPreference = 'system',
): FormattedCellValue {
  if (value == null) {
    return {
      kind: 'null',
      display: 'NULL',
    };
  }

  if (column.data_type === 'boolean') {
    const booleanValue = value === true || value === 'true' || value === 't';
    return {
      kind: 'boolean',
      display: booleanValue ? 'Yes' : 'No',
      booleanValue,
    };
  }

  if (DATE_ONLY_TYPES.has(column.data_type)) {
    const raw = String(value);
    // Append local time to prevent UTC midnight being shifted to the previous day
    // for users west of UTC (ECMAScript parses bare date strings as UTC midnight).
    const parsed = new Date(`${raw}T00:00:00`);

    if (!Number.isNaN(parsed.getTime())) {
      return {
        kind: 'timestamp',
        display: formatDate(parsed, dateFormat),
        tooltip: raw,
      };
    }
  }

  if (
    DATETIME_TYPES.has(column.data_type) ||
    column.display_type === 'datetime'
  ) {
    const raw = String(value);
    // Replace space with T for ISO 8601 compliance — Safari rejects "2024-01-15 14:30:00"
    const parsed = new Date(raw.replace(' ', 'T'));

    if (!Number.isNaN(parsed.getTime())) {
      return {
        kind: 'timestamp',
        display: formatDateTime(parsed, dateFormat),
        tooltip: raw,
      };
    }
  }

  if (NUMERIC_TEXT_TYPES.has(column.data_type)) {
    // Backend sends full-precision string — display as-is to avoid float truncation.
    return {
      kind: 'number',
      display: String(value),
    };
  }

  if (NUMBER_TYPES.has(column.data_type)) {
    const numericValue = typeof value === 'number' ? value : Number(value);
    if (Number.isFinite(numericValue)) {
      return {
        kind: 'number',
        display: numericValue.toLocaleString(),
      };
    }
  }

  return {
    kind: 'text',
    display: String(value),
  };
}

function formatDate(date: Date, dateFormat: DateFormatPreference): string {
  if (dateFormat === 'system') {
    return dateFormatter.format(date);
  }

  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');

  switch (dateFormat) {
    case 'YYYY-MM-DD':
      return `${year}-${month}-${day}`;
    case 'DD/MM/YYYY':
      return `${day}/${month}/${year}`;
    case 'MM/DD/YYYY':
      return `${month}/${day}/${year}`;
    default:
      return dateFormatter.format(date);
  }
}

function formatDateTime(date: Date, dateFormat: DateFormatPreference): string {
  if (dateFormat === 'system') {
    return formatter.format(date);
  }

  return `${formatDate(date, dateFormat)} ${timeFormatter.format(date)}`;
}

export function buildSortableColumn(
  column: ColumnInfo,
  overrides: Partial<ColumnRegular> = {},
): ColumnRegular {
  return {
    prop: column.name,
    name: getColumnDisplayName(column),
    size: columnWidth(column),
    sortable: true,
    ...overrides,
  };
}
