import type {
  ConnectionStatusResponse,
  TableInfo,
  ColumnInfo,
  QueryResult,
  DisplayConfig,
  SettingsEntries,
  TableRelationships,
  ReachableTable,
  ReferenceEntry,
  ReferencesResponse,
  TableRowResponse,
  UpdateStatus,
  VersionInfo,
  ViewColumn,
} from './types';
import { resolveViewColumnOutputNames } from './view-shape';

const MOCK_ROW_COUNT = 200;

const TABLES: TableInfo[] = [
  { schema: 'public', name: 'users', display_name: 'Users', row_count_estimate: 42 },
  {
    schema: 'public',
    name: 'activity_log',
    display_name: 'Activity Log',
    row_count_estimate: MOCK_ROW_COUNT,
  },
  { schema: 'public', name: 'events', display_name: 'Events', row_count_estimate: MOCK_ROW_COUNT },
  { schema: 'public', name: 'tickets', display_name: 'Tickets', row_count_estimate: MOCK_ROW_COUNT },
  { schema: 'public', name: 'orders', display_name: 'Orders', row_count_estimate: MOCK_ROW_COUNT },
];

const COLUMNS: Record<string, ColumnInfo[]> = {
  users: [
    {
      name: 'id',
      display_name: 'ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: true,
    },
    {
      name: 'name',
      display_name: 'Name',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'email',
      display_name: 'Email',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'role',
      display_name: 'Role',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'status',
      display_name: 'Status',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'last_login',
      display_name: 'Last Login',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: true,
      is_primary_key: false,
    },
  ],
  activity_log: [
    {
      name: 'id',
      display_name: 'ID',
      data_type: 'bigint',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: true,
    },
    {
      name: 'user_id',
      display_name: 'User ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'timestamp',
      display_name: 'Timestamp',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'action',
      display_name: 'Action',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'ip_address',
      display_name: 'IP Address',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: true,
      is_primary_key: false,
    },
    {
      name: 'details',
      display_name: 'Details',
      data_type: 'text',
      display_type: 'text',
      is_nullable: true,
      is_primary_key: false,
    },
  ],
  events: [
    {
      name: 'id',
      display_name: 'ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: true,
    },
    {
      name: 'title',
      display_name: 'Title',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'event_type',
      display_name: 'Event Type',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'severity',
      display_name: 'Severity',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'message',
      display_name: 'Message',
      data_type: 'text',
      display_type: 'text',
      is_nullable: true,
      is_primary_key: false,
    },
    {
      name: 'created_at',
      display_name: 'Created At',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: false,
      is_primary_key: false,
    },
  ],
  tickets: [
    {
      name: 'id',
      display_name: 'ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: true,
    },
    {
      name: 'user_id',
      display_name: 'User ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'priority',
      display_name: 'Priority',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'subject',
      display_name: 'Subject',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'resolved',
      display_name: 'Resolved',
      data_type: 'boolean',
      display_type: 'boolean',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'created_at',
      display_name: 'Created At',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: false,
      is_primary_key: false,
    },
  ],
  orders: [
    {
      name: 'id',
      display_name: 'ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: true,
    },
    {
      name: 'user_id',
      display_name: 'User ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'product',
      display_name: 'Product',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'amount',
      display_name: 'Amount',
      data_type: 'numeric',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'status',
      display_name: 'Status',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'created_at',
      display_name: 'Created At',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: false,
      is_primary_key: false,
    },
  ],
};

const USER_NAMES = [
  'Alice Chen',
  'Bob Wright',
  'Carol Davis',
  'Dave Patel',
  'Eve Thompson',
  'Frank Miller',
  'Grace Lee',
  'Hank Johnson',
  'Iris Wang',
  'Jack Brown',
];
const ROLES = ['admin', 'editor', 'viewer', 'manager'];
const STATUSES = ['active', 'inactive', 'suspended', 'pending'];
const ACTIONS = [
  'login',
  'logout',
  'update_profile',
  'create_record',
  'delete_record',
  'export_data',
];
const EVENT_TYPES = [
  'system_start',
  'system_stop',
  'alert',
  'maintenance',
  'deployment',
  'backup',
];
const SEVERITIES = ['info', 'warning', 'error', 'critical'];
const PRIORITIES = ['low', 'medium', 'high', 'urgent'];
const TICKET_SUBJECTS = [
  'Login issue',
  'Data export failed',
  'Permission denied',
  'Slow query',
  'Missing records',
  'UI rendering bug',
  'API timeout',
  'Incorrect totals',
];
const PRODUCTS = [
  'Widget Pro',
  'Gadget Plus',
  'Service Basic',
  'Service Premium',
  'Toolkit Starter',
];
const ORDER_STATUSES = ['pending', 'processing', 'shipped', 'delivered', 'cancelled'];

function pick<T>(arr: readonly T[]): T {
  if (arr.length === 0) throw new Error('pick called on empty array');
  return arr[Math.floor(Math.random() * arr.length)];
}

// Emits "YYYY-MM-DD HH:MM:SS", the naive form the backend sends for a timestamp column.
// An ISO string with a "Z" suffix would name an instant, which the naive columns never do.
function randomTimestamp(daysBack: number): string {
  const now = Date.now();
  const offset = Math.floor(Math.random() * daysBack * 86400000);
  return new Date(now - offset).toISOString().slice(0, 19).replace('T', ' ');
}

function randomIp(): string {
  return `${10 + Math.floor(Math.random() * 240)}.${Math.floor(Math.random() * 256)}.${Math.floor(Math.random() * 256)}.${Math.floor(Math.random() * 256)}`;
}

function generateUserRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    name: USER_NAMES[i % USER_NAMES.length],
    email: `${USER_NAMES[i % USER_NAMES.length].toLowerCase().replace(' ', '.')}@example.com`,
    role: pick(ROLES),
    status: pick(STATUSES),
    last_login: randomTimestamp(7),
  }));
}

function generateActivityLogRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    user_id: 1 + Math.floor(Math.random() * 10),
    timestamp: randomTimestamp(30),
    action: pick(ACTIONS),
    ip_address: randomIp(),
    details: `User performed ${pick(ACTIONS)} action`,
  }));
}

function generateEventRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    title: `Event ${i + 1}`,
    event_type: pick(EVENT_TYPES),
    severity: pick(SEVERITIES),
    message: `${pick(EVENT_TYPES)} event triggered at ${randomTimestamp(1)}`,
    created_at: randomTimestamp(60),
  }));
}

function generateTicketRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    user_id: 1 + Math.floor(Math.random() * 10),
    priority: pick(PRIORITIES),
    subject: pick(TICKET_SUBJECTS),
    resolved: Math.random() > 0.3,
    created_at: randomTimestamp(90),
  }));
}

function generateOrderRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    user_id: 1 + Math.floor(Math.random() * 10),
    product: pick(PRODUCTS),
    amount: Math.round((5 + Math.random() * 495) * 100) / 100,
    status: pick(ORDER_STATUSES),
    created_at: randomTimestamp(30),
  }));
}

const ROW_GENERATORS: Record<
  string,
  (count: number) => Record<string, unknown>[]
> = {
  users: generateUserRows,
  activity_log: generateActivityLogRows,
  events: generateEventRows,
  tickets: generateTicketRows,
  orders: generateOrderRows,
};

const ROW_CACHE: Record<string, Record<string, unknown>[]> = {};

function getRows(table: string, count: number): Record<string, unknown>[] {
  if (!ROW_CACHE[table]) {
    const generator = ROW_GENERATORS[table];
    ROW_CACHE[table] = generator ? generator(count) : [];
  }
  return ROW_CACHE[table];
}

export function mockFetchTables(): TableInfo[] {
  return TABLES;
}

export function mockFetchColumns(_schema: string, table: string): ColumnInfo[] {
  return COLUMNS[table] ?? [];
}

// FK fixtures for mock mode. Cover a plain FK, an incoming edge, a composite
// FK, and a target outside the allowlist so the grid hides its hop affordance.
const RELATIONSHIPS: Record<string, TableRelationships> = {
  orders: {
    outgoing: [
      {
        constraint: 'orders_user_id_fkey',
        columns: ['user_id'],
        target: {
          schema: 'public',
          table: 'users',
          display_name: 'Users',
          columns: ['id'],
          allowed: true,
        },
      },
      {
        constraint: 'orders_warehouse_fkey',
        columns: ['warehouse_region', 'warehouse_code'],
        target: {
          schema: 'public',
          table: 'warehouses',
          display_name: 'Warehouses',
          columns: ['region', 'code'],
          allowed: false,
        },
      },
    ],
    incoming: [],
  },
  tickets: {
    outgoing: [
      {
        constraint: 'tickets_user_id_fkey',
        columns: ['user_id'],
        target: {
          schema: 'public',
          table: 'users',
          display_name: 'Users',
          columns: ['id'],
          allowed: true,
        },
      },
    ],
    incoming: [],
  },
  users: {
    outgoing: [],
    incoming: [
      {
        constraint: 'orders_user_id_fkey',
        columns: ['id'],
        source: {
          schema: 'public',
          table: 'orders',
          display_name: 'Orders',
          columns: ['user_id'],
          allowed: true,
        },
      },
      {
        constraint: 'tickets_user_id_fkey',
        columns: ['id'],
        source: {
          schema: 'public',
          table: 'tickets',
          display_name: 'Tickets',
          columns: ['user_id'],
          allowed: true,
        },
      },
    ],
  },
};

export function mockFetchTableRelationships(
  _schema: string,
  table: string,
): TableRelationships {
  return RELATIONSHIPS[table] ?? { outgoing: [], incoming: [] };
}

// Bulk FK reachability fixtures, consistent with RELATIONSHIPS above.
// `warehouses` never appears: its only edge (from orders) is outside the allowlist.
const REACHABLE: Record<string, ReachableTable[]> = {
  orders: [
    { schema: 'public', table: 'users', display_name: 'Users', hops: 1 },
    { schema: 'public', table: 'tickets', display_name: 'Tickets', hops: 2 },
  ],
  tickets: [
    { schema: 'public', table: 'users', display_name: 'Users', hops: 1 },
    { schema: 'public', table: 'orders', display_name: 'Orders', hops: 2 },
  ],
  users: [
    { schema: 'public', table: 'orders', display_name: 'Orders', hops: 1 },
    { schema: 'public', table: 'tickets', display_name: 'Tickets', hops: 1 },
  ],
};

export function mockFetchFkReachable(
  _schema: string,
  table: string,
): ReachableTable[] {
  return REACHABLE[table] ?? [];
}

/**
 * Capped counts of referencing rows, derived from the same generated mock
 * rows `mockFetchRows` reads from. Covers users referenced by orders and
 * tickets, matching the `RELATIONSHIPS.users.incoming` fixture.
 */
export function mockFetchTableReferences(
  _schema: string,
  table: string,
  exactFilters: Record<string, string>,
): ReferencesResponse {
  const relationships = RELATIONSHIPS[table];
  if (!relationships) return { references: [] };

  const references: ReferenceEntry[] = [];
  for (const edge of relationships.incoming) {
    const values = edge.columns.map((col) => exactFilters[col]);
    if (values.some((value) => value == null)) continue;

    const sourceTable = edge.source.table;
    const info = TABLES.find((t) => t.name === sourceTable);
    const totalRows = info?.row_count_estimate ?? 0;
    const allRows = getRows(sourceTable, totalRows);
    const count = allRows.filter((row) =>
      edge.source.columns.every((col, i) => String(row[col] ?? '') === values[i]),
    ).length;

    references.push({
      schema: edge.source.schema,
      table: sourceTable,
      display_name: edge.source.display_name,
      columns: edge.source.columns,
      count: Math.min(count, 1000),
      capped: count > 1000,
    });
  }

  references.sort((a, b) => a.display_name.localeCompare(b.display_name));
  return { references };
}

export function mockFetchRows(
  _schema: string,
  table: string,
  params?: {
    page?: number;
    page_size?: number;
    sort?: string;
    search?: string;
    filters?: Record<string, string>;
    exact_filters?: Record<string, string>;
  },
): QueryResult {
  const page = params?.page ?? 1;
  const pageSize = params?.page_size ?? 50;
  const info = TABLES.find((t) => t.name === table);
  const totalRows = info?.row_count_estimate ?? 50;

  const allRows = getRows(table, totalRows);
  let rows = [...allRows];

  if (params?.search) {
    const q = params.search.toLowerCase();
    rows = rows.filter((row) =>
      Object.values(row).some((v) =>
        String(v ?? '').toLowerCase().includes(q),
      ),
    );
  }

  if (params?.filters) {
    const activeFilters = Object.entries(params.filters).filter(
      ([, value]) => value.trim().length > 0,
    );

    if (activeFilters.length > 0) {
      rows = rows.filter((row) =>
        activeFilters.every(([column, value]) =>
          String(row[column] ?? '').toLowerCase().includes(value.toLowerCase()),
        ),
      );
    }
  }

  if (params?.exact_filters) {
    const exactEntries = Object.entries(params.exact_filters);
    if (exactEntries.length > 0) {
      rows = rows.filter((row) =>
        exactEntries.every(
          ([column, value]) => String(row[column] ?? '') === value,
        ),
      );
    }
  }

  const sortEntries = parseSortParam(params?.sort);
  if (sortEntries.length > 0) {
    rows.sort((a, b) => compareRowsBySortState(a, b, sortEntries));
  }

  const filteredTotal = rows.length;
  const start = (page - 1) * pageSize;
  const paged = rows.slice(start, start + pageSize);

  return {
    columns: COLUMNS[table] ?? [],
    rows: paged,
    total_rows: filteredTotal,
    page,
    page_size: pageSize,
  };
}

export type MockTransientViewQueryColumn = ViewColumn;

export interface MockTransientViewQueryBody {
  base_schema: string;
  base_table: string;
  shape: { columns: MockTransientViewQueryColumn[] };
  page?: number;
  page_size?: number;
  sort?: string;
  search?: string;
  filters?: Record<string, string>;
  exact_filters?: Record<string, string>;
}

/**
 * Mock support for POST /api/views/query. Covers the orders -> users join
 * fixture only, joining on `user_id`, which is enough for mock mode to
 * render a picked user column inline on the orders table.
 */
export function mockFetchTransientViewRows(
  body: MockTransientViewQueryBody,
): QueryResult {
  const baseResult = mockFetchRows(body.base_schema, body.base_table, {
    page: body.page,
    page_size: body.page_size,
    sort: body.sort,
    search: body.search,
    filters: body.filters,
  });

  const usersById =
    body.base_table === 'orders'
      ? new Map(getRows('users', TABLES.find((t) => t.name === 'users')?.row_count_estimate ?? 50)
          .map((row) => [Number(row.id), row] as const))
      : new Map<number, Record<string, unknown>>();

  // Mock mode shares the real output-name resolution, so a collision renames
  // a column here exactly as the backend renames it.
  const outputNames = resolveViewColumnOutputNames(body.shape.columns);

  const rows = baseResult.rows.map((row) => {
    const related = usersById.get(Number(row.user_id));
    const output: Record<string, unknown> = {};
    body.shape.columns.forEach((column, index) => {
      const outputName = outputNames[index];
      output[outputName] =
        column.source_table === body.base_table
          ? row[column.column_name]
          : (related?.[column.column_name] ?? null);
    });
    return output;
  });

  const columns: ColumnInfo[] = body.shape.columns.map((column, index) => {
    const sourceColumns = COLUMNS[column.source_table] ?? [];
    const info = sourceColumns.find((c) => c.name === column.column_name);
    return {
      name: outputNames[index],
      display_name: info?.display_name ?? column.column_name,
      data_type: info?.data_type ?? 'text',
      display_type: info?.display_type ?? 'text',
      is_nullable: info?.is_nullable ?? true,
      is_primary_key: false,
    };
  });

  return {
    columns,
    rows,
    total_rows: baseResult.total_rows,
    page: baseResult.page,
    page_size: baseResult.page_size,
  };
}

/**
 * Mock counterpart of GET /tables/{schema}/{table}/row, used by the FK peek
 * panel. Looks up the generated fixture rows for the table and returns the
 * first exact match. Mock mode never generates a second colliding row, so
 * `multiple` is always false here.
 */
export function mockFetchTableRow(
  _schema: string,
  table: string,
  exactFilters: Record<string, string>,
): TableRowResponse {
  const info = TABLES.find((t) => t.name === table);
  const totalRows = info?.row_count_estimate ?? 50;
  const allRows = getRows(table, totalRows);
  const entries = Object.entries(exactFilters);

  const row =
    entries.length === 0
      ? null
      : (allRows.find((candidate) =>
          entries.every(([column, value]) => String(candidate[column] ?? '') === value),
        ) ?? null);

  return {
    row,
    multiple: false,
    columns: COLUMNS[table] ?? [],
  };
}

type MockSortEntry = {
  column: string;
  direction: 'asc' | 'desc';
};

function parseSortParam(sort?: string): MockSortEntry[] {
  if (sort == null) {
    return [];
  }

  const trimmed = sort.trim();
  if (trimmed.length === 0) {
    return [];
  }

  return trimmed.split(',').flatMap((segment) => {
    const cleaned = segment.trim();
    if (cleaned.length === 0) {
      return [];
    }

    const [columnPart, directionPart] = cleaned.split(':');
    if (!columnPart || !directionPart) {
      return [];
    }

    const column = columnPart.trim();
    const direction = directionPart.trim().toLowerCase();
    if (!column || (direction !== 'asc' && direction !== 'desc')) {
      return [];
    }

    return [{ column, direction }];
  });
}

function compareScalarValues(a: unknown, b: unknown): number {
  if (a == null && b == null) return 0;
  if (a == null) return 1;
  if (b == null) return -1;
  const left = a as any;
  const right = b as any;
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

function compareRowsBySortState(
  a: Record<string, unknown>,
  b: Record<string, unknown>,
  sortEntries: MockSortEntry[],
): number {
  for (const { column, direction } of sortEntries) {
    const result = compareScalarValues(a[column], b[column]);
    if (result !== 0) {
      return direction === 'asc' ? result : -result;
    }
  }

  return 0;
}

export function mockFetchDisplayConfig(): DisplayConfig {
  return {
    branding: {
      title: 'SeeKi',
      subtitle: 'Database Viewer',
    },
    timezone: 'UTC',
    tables: Object.fromEntries(
      TABLES.map((t) => [
        `${t.schema}.${t.name}`,
        {
          display_name: t.display_name,
          columns: Object.fromEntries(
            (COLUMNS[t.name] ?? []).map((c) => [
              c.name,
              { display_name: c.display_name },
            ]),
          ),
        },
      ]),
    ),
  };
}

export function mockFetchSettings(): SettingsEntries {
  return {
    'appearance.date_format': 'system',
    'appearance.row_density': 'comfortable',
  };
}

export function mockFetchConnectionStatus(): ConnectionStatusResponse {
  return {
    database_kind: 'postgres',
    host: 'db.internal',
    port: 5432,
    database: 'fleet',
    schemas: ['public', 'reporting'],
    ssh_enabled: true,
    ssh_connected: true,
  };
}

export function mockFetchVersion(): VersionInfo {
  return {
    version: '26.5.0.3a',
    commit: 'abc123def456',
    built_at: '2026-04-15T00:00:00Z',
  };
}

export function mockFetchUpdateStatus(): UpdateStatus | null {
  return {
    current: '26.5.0.3a',
    latest: '26.5.0.3n260416g1a2b3c4',
    pre_release_channel: false,
    poll_interval_hours: 6,
    update_available: true,
    previous_exists: false,
    last_checked: '2026-04-16T09:00:00Z',
    release_notes: '## Mock release\n\n- Added update polling\n- Added release badges',
    available_builds: [],
  };
}
