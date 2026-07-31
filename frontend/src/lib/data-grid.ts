import type { ColumnRegular } from '@revolist/svelte-datagrid';
import type { ColumnInfo, DateFormatPreference, SortDirection, SortState } from './types';

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
// 'money' included here so non-null money values right-align (kind:'number') to match
// null money cells already right-aligned via isNumericCol in DataGrid.svelte.
const NUMERIC_TEXT_TYPES = new Set([
  'numeric',
  'money',
]);

const DATE_ONLY_TYPES = new Set([
  'date',
]);

const DATETIME_TYPES = new Set([
  'timestamp',
  'timestamp without time zone',
  'timestamp with time zone',
]);

// The zone every timestamp renders in, set once from /api/config/display so the
// grid, the tooltip, the CSV export and SQL-side date buckets all agree. Until
// it arrives, `undefined` keeps Intl on the viewer's local zone — the previous
// behaviour, and the only sensible guess before the backend has told us.
let displayTimeZone: string | undefined;

// Intl.DateTimeFormat construction is not free and these are hit per cell, so
// they are cached and only rebuilt when the configured zone changes.
let dateFormatter: Intl.DateTimeFormat;
let formatter: Intl.DateTimeFormat;
let timeFormatter: Intl.DateTimeFormat;
let partsFormatter: Intl.DateTimeFormat;

function buildFormatters(timeZone: string | undefined): void {
  dateFormatter = new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    timeZone,
  });
  formatter = new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    timeZone,
  });
  timeFormatter = new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
    timeZone,
  });
  // Explicit-format branches need the calendar date *in the display zone*, not
  // the browser's — date.getFullYear() would silently reintroduce local time.
  partsFormatter = new Intl.DateTimeFormat('en-CA', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    timeZone,
  });
}

buildFormatters(undefined);

// Deliberately zone-less and never rebuilt: offset-less `timestamp` values are
// rendered from a Date whose *local* parts hold the literal wall-clock, so
// applying any timeZone here would shift them back off again.
const naiveFormatter = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
  hour: 'numeric',
  minute: '2-digit',
});

const naiveTimeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: 'numeric',
  minute: '2-digit',
});

/**
 * Pin the zone used to render every timestamp. Called once with the value from
 * /api/config/display. An unrecognised zone is ignored rather than thrown, so a
 * bad config degrades to local time instead of blanking the grid.
 */
export function setDisplayTimeZone(timeZone: string | undefined): void {
  if (timeZone === displayTimeZone) return;
  try {
    new Intl.DateTimeFormat(undefined, { timeZone }).format(new Date());
  } catch {
    console.warn(`Ignoring unrecognised display timezone: ${timeZone}`);
    return;
  }
  displayTimeZone = timeZone;
  buildFormatters(timeZone);
}

/** The zone currently used for rendering, or undefined for the viewer's local zone. */
export function getDisplayTimeZone(): string | undefined {
  return displayTimeZone;
}

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
): Record<string, SortDirection> | undefined {
  if (sortState.length === 0) {
    return undefined;
  }

  return Object.fromEntries(
    sortState.map((entry) => [entry.column, entry.direction]),
  ) as Record<string, SortDirection>;
}

export function cycleSort(sortState: SortState, column: string): SortState {
  const index = sortState.findIndex((entry) => entry.column === column);

  if (index === -1) {
    return [{ column, direction: 'asc' }, ...sortState];
  }

  const current = sortState[index];
  if (current.direction === 'asc') {
    return [
      { column, direction: 'desc' },
      ...sortState.slice(0, index),
      ...sortState.slice(index + 1),
    ];
  }

  return sortState.filter((entry) => entry.column !== column);
}

// Single-column cycle: replaces any existing multi-sort with a cycle of just this column.
// Used for non-shift clicks so users can return to a single-column sort after stacking.
export function replaceSort(sortState: SortState, column: string): SortState {
  const current = sortState.find((entry) => entry.column === column);
  if (!current) return [{ column, direction: 'asc' }];
  if (current.direction === 'asc') return [{ column, direction: 'desc' }];
  return [];
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
    const isoish = raw.replace(' ', 'T');
    // A `timestamptz` arrives with an explicit offset and is a real instant, so
    // it renders in the configured display zone. A naive `timestamp` has no
    // offset and is already wall-clock in that zone — reformatting it through
    // Intl would shift it by the browser-vs-display zone difference, so its
    // parts are shown verbatim.
    const parsed = new Date(isoish);
    if (Number.isNaN(parsed.getTime())) {
      return { kind: 'text', display: raw };
    }

    return {
      kind: 'timestamp',
      display: hasExplicitOffset(isoish)
        ? formatDateTime(parsed, dateFormat)
        : formatNaiveDateTime(isoish, dateFormat),
      tooltip: raw,
    };
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
    display: stringifyCellValue(value),
  };
}

// Render any remaining value as text. Objects and arrays (e.g. json/jsonb columns)
// would otherwise stringify to "[object Object]" via String(); show their JSON text
// instead so the cell reads like the varchar fallback the rest of the grid uses.
function stringifyCellValue(value: unknown): string {
  if (typeof value === 'object') {
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  return String(value);
}

function formatDate(date: Date, dateFormat: DateFormatPreference): string {
  if (dateFormat === 'system') {
    return dateFormatter.format(date);
  }

  // en-CA yields "YYYY-MM-DD", giving the calendar parts as seen in the display
  // zone. Reading them off the Date directly would use the browser's zone and
  // could name a different day than the 'system' branch above.
  const [year, month, day] = partsFormatter.format(date).split('-');

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

/** Matches a trailing `Z`, `+HH:MM`, `-HH:MM`, `+HHMM` or `+HH` on the time part. */
const EXPLICIT_OFFSET = /(?:Z|[+-]\d{2}(?::?\d{2})?)$/;

function hasExplicitOffset(isoish: string): boolean {
  const separator = isoish.indexOf('T');
  // No time component at all means no offset. Scanning the whole string here
  // would read the "-15" of "2024-01-15" as a -15:00 offset.
  if (separator === -1) return false;
  return EXPLICIT_OFFSET.test(isoish.slice(separator + 1));
}

/**
 * Render an offset-less `timestamp` using its own literal parts.
 *
 * These values are wall-clock in the configured display zone already, so they
 * are reassembled rather than pushed through Intl — which would interpret them
 * as browser-local and shift them.
 */
function formatNaiveDateTime(
  isoish: string,
  dateFormat: DateFormatPreference,
): string {
  const [datePart, timePart = ''] = isoish.split('T');
  const [year, month, day] = datePart.split('-');
  const [hour = '00', minute = '00'] = timePart.split(':');
  if (!year || !month || !day) return isoish;

  // Reuse the shared formatters by handing them a Date whose *local* parts equal
  // the literal ones, so 'system' output stays locale-shaped (e.g. "30 Jul 2026,
  // 2:30 pm") without any zone conversion applied.
  const asLocal = new Date(
    Number(year),
    Number(month) - 1,
    Number(day),
    Number(hour),
    Number(minute),
  );
  if (Number.isNaN(asLocal.getTime())) return isoish;

  if (dateFormat === 'system') {
    return naiveFormatter.format(asLocal);
  }

  const date =
    dateFormat === 'YYYY-MM-DD'
      ? `${year}-${month}-${day}`
      : dateFormat === 'DD/MM/YYYY'
        ? `${day}/${month}/${year}`
        : `${month}/${day}/${year}`;
  return `${date} ${naiveTimeFormatter.format(asLocal)}`;
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
