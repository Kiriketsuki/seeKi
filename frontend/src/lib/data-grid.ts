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

// Values of this type arrive as RFC 3339 strings that carry a real offset, so the
// frontend resolves the exact instant and renders it in the configured display zone.
const TIMESTAMPTZ_TYPES = new Set([
  'timestamp with time zone',
]);

// Values of these types carry no offset. The frontend renders the wall clock verbatim
// and applies no zone conversion.
const NAIVE_DATETIME_TYPES = new Set([
  'timestamp',
  'timestamp without time zone',
]);

// Matches "YYYY-MM-DD HH:MM[:SS[.fff]]" and the T-separated form. The pattern anchors at
// both ends, so a string that carries a "Z" or a "+HH:MM" offset does not match here and
// takes the instant path instead. The capture groups render the wall clock verbatim.
const NAIVE_DATETIME_PATTERN =
  /^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2}))?(?:\.\d+)?$/;

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

// Intl.DateTimeFormat construction costs real time and formatCellValue runs per cell.
// This cache keeps one formatter per (timezone, dateFormat) pair.
const tzFormatters = new Map<string, Intl.DateTimeFormat>();

function getZonedFormatter(timezone: string): Intl.DateTimeFormat {
  const cached = tzFormatters.get(timezone);
  if (cached) return cached;

  let built: Intl.DateTimeFormat;
  try {
    built = new Intl.DateTimeFormat(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
      timeZone: timezone,
    });
  } catch {
    // The backend validates the zone name at config load, so this branch is defense in
    // depth only. Fall back to the viewer local zone rather than throw during render.
    built = formatter;
  }

  tzFormatters.set(timezone, built);
  return built;
}

const tzTimeFormatters = new Map<string, Intl.DateTimeFormat>();

// Mirrors timeFormatter, with the display zone applied.
function getZonedTimeFormatter(timezone: string): Intl.DateTimeFormat {
  const cached = tzTimeFormatters.get(timezone);
  if (cached) return cached;

  let built: Intl.DateTimeFormat;
  try {
    built = new Intl.DateTimeFormat(undefined, {
      hour: 'numeric',
      minute: '2-digit',
      timeZone: timezone,
    });
  } catch {
    built = timeFormatter;
  }

  tzTimeFormatters.set(timezone, built);
  return built;
}

interface ZonedParts {
  year: string;
  month: string;
  day: string;
  hour: string;
  minute: string;
}

// Reads the calendar parts of an instant in the target zone. Date.getFullYear and its
// siblings would report the viewer local zone, which reintroduces the shift this fixes.
const tzPartsFormatters = new Map<string, Intl.DateTimeFormat | null>();

// Mirrors tzFormatters. The options are fixed apart from the zone, so one formatter per
// zone serves every cell and formatCellValue pays no construction cost per cell.
function getZonedPartsFormatter(timezone: string): Intl.DateTimeFormat | null {
  if (tzPartsFormatters.has(timezone)) {
    return tzPartsFormatters.get(timezone) ?? null;
  }

  let built: Intl.DateTimeFormat | null;
  try {
    built = new Intl.DateTimeFormat('en-GB', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      hourCycle: 'h23',
      timeZone: timezone,
    });
  } catch {
    built = null;
  }

  tzPartsFormatters.set(timezone, built);
  return built;
}

function zonedParts(date: Date, timezone: string): ZonedParts | null {
  const formatterForZone = getZonedPartsFormatter(timezone);
  if (!formatterForZone) return null;

  const parts = formatterForZone.formatToParts(date);

  const lookup = (type: string): string =>
    parts.find((p) => p.type === type)?.value ?? '';

  return {
    year: lookup('year'),
    month: lookup('month'),
    day: lookup('day'),
    hour: lookup('hour'),
    minute: lookup('minute'),
  };
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
      return 110;
    // The backend appends the stored offset, for example "14:30:00+08:00", so this
    // column needs about seven extra characters of room.
    case 'time with time zone':
      return 165;
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
  timezone: string = 'UTC',
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

  if (TIMESTAMPTZ_TYPES.has(column.data_type)) {
    const raw = String(value);
    // The string carries its own offset, so new Date resolves the exact instant. Intl
    // then renders that instant in the display zone and handles daylight saving.
    const parsed = new Date(raw);

    if (!Number.isNaN(parsed.getTime())) {
      return {
        kind: 'timestamp',
        display: formatZonedDateTime(parsed, dateFormat, timezone),
        tooltip: raw,
      };
    }
  }

  if (
    NAIVE_DATETIME_TYPES.has(column.data_type) ||
    column.display_type === 'datetime'
  ) {
    const raw = String(value);
    // The string carries no offset. Render the captured parts verbatim and pin every Intl
    // call to UTC. The local-time Date constructor would shift a wall clock that lands in
    // the viewer own daylight saving gap, for example 02:30 on a spring-forward date.
    const match = NAIVE_DATETIME_PATTERN.exec(raw);

    if (match) {
      const display = formatNaiveDateTime(match, dateFormat);
      if (display) {
        return {
          kind: 'timestamp',
          display,
          tooltip: raw,
        };
      }
    }

    // The value carries an offset or a "Z" suffix, so it names a real instant. Resolve it
    // and render it in the display zone, the same way a timestamptz column renders.
    const parsed = new Date(raw);
    if (!Number.isNaN(parsed.getTime())) {
      return {
        kind: 'timestamp',
        display: formatZonedDateTime(parsed, dateFormat, timezone),
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

// Renders a naive wall clock from its captured parts. No zone conversion happens. The
// "system" branch needs Intl for the locale pattern, so it feeds Intl a UTC instant and a
// UTC-pinned formatter. UTC has no daylight saving, so the parts survive the round trip.
function formatNaiveDateTime(
  match: RegExpExecArray,
  dateFormat: DateFormatPreference,
): string | null {
  const year = match[1];
  const month = match[2];
  const day = match[3];
  const hour = match[4];
  const minute = match[5];

  const instant = new Date(
    Date.UTC(
      Number(year),
      Number(month) - 1,
      Number(day),
      Number(hour),
      Number(minute),
      Number(match[6] ?? '0'),
    ),
  );

  if (Number.isNaN(instant.getTime())) {
    return null;
  }

  if (dateFormat === 'system') {
    return getZonedFormatter('UTC').format(instant);
  }

  const time = getZonedTimeFormatter('UTC').format(instant);

  switch (dateFormat) {
    case 'YYYY-MM-DD':
      return `${year}-${month}-${day} ${time}`;
    case 'DD/MM/YYYY':
      return `${day}/${month}/${year} ${time}`;
    case 'MM/DD/YYYY':
      return `${month}/${day}/${year} ${time}`;
    default:
      return getZonedFormatter('UTC').format(instant);
  }
}

function formatZonedDateTime(
  date: Date,
  dateFormat: DateFormatPreference,
  timezone: string,
): string {
  if (dateFormat === 'system') {
    return getZonedFormatter(timezone).format(date);
  }

  const parts = zonedParts(date, timezone);
  if (!parts) {
    return formatDateTime(date, dateFormat);
  }

  const time = getZonedTimeFormatter(timezone).format(date);

  switch (dateFormat) {
    case 'YYYY-MM-DD':
      return `${parts.year}-${parts.month}-${parts.day} ${time}`;
    case 'DD/MM/YYYY':
      return `${parts.day}/${parts.month}/${parts.year} ${time}`;
    case 'MM/DD/YYYY':
      return `${parts.month}/${parts.day}/${parts.year} ${time}`;
    default:
      return getZonedFormatter(timezone).format(date);
  }
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
