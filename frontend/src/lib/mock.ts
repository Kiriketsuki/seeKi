import type {
  TableInfo,
  ColumnInfo,
  QueryResult,
  DisplayConfig,
} from './types';

const TABLES: TableInfo[] = [
  { name: 'vehicles', display_name: 'Vehicles', row_count_estimate: 42 },
  {
    name: 'vehicles_log',
    display_name: 'Vehicles Log',
    row_count_estimate: 427229,
  },
  { name: 'events', display_name: 'Events', row_count_estimate: 18000 },
  { name: 'faults', display_name: 'Faults', row_count_estimate: 3200 },
  { name: 'flights', display_name: 'Flights', row_count_estimate: 890 },
];

const COLUMNS: Record<string, ColumnInfo[]> = {
  vehicles: [
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
      name: 'type',
      display_name: 'Type',
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
      name: 'firmware_version',
      display_name: 'Firmware Version',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: true,
      is_primary_key: false,
    },
    {
      name: 'last_seen',
      display_name: 'Last Seen',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: true,
      is_primary_key: false,
    },
  ],
  vehicles_log: [
    {
      name: 'id',
      display_name: 'ID',
      data_type: 'bigint',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: true,
    },
    {
      name: 'vehicle_id',
      display_name: 'Vehicle ID',
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
      name: 'supervisor',
      display_name: 'Supervisor',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: true,
      is_primary_key: false,
    },
    {
      name: 'posn_lat',
      display_name: 'Latitude',
      data_type: 'double precision',
      display_type: 'number',
      is_nullable: true,
      is_primary_key: false,
    },
    {
      name: 'posn_lon',
      display_name: 'Longitude',
      data_type: 'double precision',
      display_type: 'number',
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
      name: 'vehicle_id',
      display_name: 'Vehicle ID',
      data_type: 'integer',
      display_type: 'number',
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
  faults: [
    {
      name: 'id',
      display_name: 'ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: true,
    },
    {
      name: 'vehicle_id',
      display_name: 'Vehicle ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'fault_code',
      display_name: 'Fault Code',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'description',
      display_name: 'Description',
      data_type: 'text',
      display_type: 'text',
      is_nullable: true,
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
      name: 'reported_at',
      display_name: 'Reported At',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: false,
      is_primary_key: false,
    },
  ],
  flights: [
    {
      name: 'id',
      display_name: 'ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: true,
    },
    {
      name: 'vehicle_id',
      display_name: 'Vehicle ID',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'route_name',
      display_name: 'Route Name',
      data_type: 'varchar',
      display_type: 'text',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'departure_time',
      display_name: 'Departure Time',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: false,
      is_primary_key: false,
    },
    {
      name: 'arrival_time',
      display_name: 'Arrival Time',
      data_type: 'timestamp',
      display_type: 'datetime',
      is_nullable: true,
      is_primary_key: false,
    },
    {
      name: 'passenger_count',
      display_name: 'Passengers',
      data_type: 'integer',
      display_type: 'number',
      is_nullable: true,
      is_primary_key: false,
    },
  ],
};

const VEHICLE_NAMES = [
  'AV-001',
  'AV-002',
  'AV-003',
  'AV-004',
  'AV-005',
  'AV-006',
  'AV-007',
  'AV-008',
  'AV-009',
  'AV-010',
];
const VEHICLE_TYPES = ['shuttle', 'pod', 'cargo'];
const STATUSES = ['active', 'idle', 'maintenance', 'offline'];
const SUPERVISORS = [
  'Alice Chen',
  'Bob Wright',
  'Carol Davis',
  'Dave Patel',
  'Eve Thompson',
];
const EVENT_TYPES = [
  'start',
  'stop',
  'obstacle_detected',
  'route_change',
  'emergency_stop',
  'passenger_boarding',
];
const SEVERITIES = ['info', 'warning', 'error', 'critical'];
const FAULT_CODES = [
  'LIDAR-001',
  'GPS-002',
  'MOTOR-003',
  'BATT-004',
  'COMM-005',
  'BRAKE-006',
  'STEER-007',
  'CAM-008',
];
const ROUTES = [
  'Campus Loop A',
  'Campus Loop B',
  'Terminal Shuttle',
  'Parking Transfer',
  'Perimeter Route',
];

function pick<T>(arr: readonly T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

function randomTimestamp(daysBack: number): string {
  const now = Date.now();
  const offset = Math.floor(Math.random() * daysBack * 86400000);
  return new Date(now - offset).toISOString();
}

function generateVehicleRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    name: VEHICLE_NAMES[i % VEHICLE_NAMES.length],
    type: pick(VEHICLE_TYPES),
    status: pick(STATUSES),
    firmware_version: `${1 + Math.floor(Math.random() * 3)}.${Math.floor(Math.random() * 10)}.${Math.floor(Math.random() * 20)}`,
    last_seen: randomTimestamp(7),
  }));
}

function generateVehiclesLogRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    vehicle_id: 1 + Math.floor(Math.random() * 10),
    timestamp: randomTimestamp(30),
    supervisor: pick(SUPERVISORS),
    posn_lat: 52.04 + Math.random() * 0.02,
    posn_lon: -1.015 + Math.random() * 0.02,
  }));
}

function generateEventRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    vehicle_id: 1 + Math.floor(Math.random() * 10),
    event_type: pick(EVENT_TYPES),
    severity: pick(SEVERITIES),
    message: `Event ${i + 1}: ${pick(EVENT_TYPES)} on vehicle ${1 + Math.floor(Math.random() * 10)}`,
    created_at: randomTimestamp(60),
  }));
}

function generateFaultRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    vehicle_id: 1 + Math.floor(Math.random() * 10),
    fault_code: pick(FAULT_CODES),
    description: `Fault detected in subsystem: ${pick(FAULT_CODES).split('-')[0].toLowerCase()}`,
    resolved: Math.random() > 0.3,
    reported_at: randomTimestamp(90),
  }));
}

function generateFlightRows(count: number): Record<string, unknown>[] {
  return Array.from({ length: count }, (_, i) => {
    const dep = randomTimestamp(30);
    const depMs = new Date(dep).getTime();
    const arrivalMs = depMs + (5 + Math.floor(Math.random() * 25)) * 60000;
    return {
      id: i + 1,
      vehicle_id: 1 + Math.floor(Math.random() * 10),
      route_name: pick(ROUTES),
      departure_time: dep,
      arrival_time: Math.random() > 0.1 ? new Date(arrivalMs).toISOString() : null,
      passenger_count: Math.floor(Math.random() * 12),
    };
  });
}

const ROW_GENERATORS: Record<
  string,
  (count: number) => Record<string, unknown>[]
> = {
  vehicles: generateVehicleRows,
  vehicles_log: generateVehiclesLogRows,
  events: generateEventRows,
  faults: generateFaultRows,
  flights: generateFlightRows,
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

export function mockFetchColumns(table: string): ColumnInfo[] {
  return COLUMNS[table] ?? [];
}

export function mockFetchRows(
  table: string,
  params?: {
    page?: number;
    page_size?: number;
    sort_column?: string;
    sort_direction?: string;
    search?: string;
  },
): QueryResult {
  const page = params?.page ?? 1;
  const pageSize = params?.page_size ?? 50;
  const info = TABLES.find((t) => t.name === table);
  const totalRows = info?.row_count_estimate ?? 50;

  const allRows = getRows(table, 50);
  let rows = [...allRows];

  if (params?.search) {
    const q = params.search.toLowerCase();
    rows = rows.filter((row) =>
      Object.values(row).some((v) =>
        String(v ?? '').toLowerCase().includes(q),
      ),
    );
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

  const start = (page - 1) * pageSize;
  const paged = rows.slice(start, start + pageSize);

  return {
    columns: COLUMNS[table] ?? [],
    rows: paged,
    total_rows: totalRows,
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
        t.name,
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
