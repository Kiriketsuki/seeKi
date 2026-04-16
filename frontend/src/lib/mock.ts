import type {
  ConnectionStatusResponse,
  TableInfo,
  ColumnInfo,
  QueryResult,
  DisplayConfig,
  SettingsEntries,
  UpdateStatusResponse,
  VersionResponse,
} from './types';

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

function randomTimestamp(daysBack: number): string {
  const now = Date.now();
  const offset = Math.floor(Math.random() * daysBack * 86400000);
  return new Date(now - offset).toISOString();
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

export function mockFetchRows(
  _schema: string,
  table: string,
  params?: {
    page?: number;
    page_size?: number;
    sort_column?: string;
    sort_direction?: string;
    search?: string;
    filters?: Record<string, string>;
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

  if (params?.sort_column) {
    const col = params.sort_column;
    const dir = params?.sort_direction === 'desc' ? -1 : 1;
    rows.sort((a, b) => {
      const av = a[col];
      const bv = b[col];
      if (av == null && bv == null) return 0;
      if (av == null) return 1;
      if (bv == null) return -1;
      if (av < bv) return -dir;
      if (av > bv) return dir;
      return 0;
    });
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

export function mockFetchDisplayConfig(): DisplayConfig {
  return {
    branding: {
      title: 'SeeKi',
      subtitle: 'Database Viewer',
    },
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

export function mockFetchVersion(): VersionResponse {
  return {
    version: '26.5.0.3a',
    commit: 'abc123def456',
    built_at: '2026-04-15T00:00:00Z',
  };
}

export function mockFetchUpdateStatus(): UpdateStatusResponse | null {
  return null;
}
