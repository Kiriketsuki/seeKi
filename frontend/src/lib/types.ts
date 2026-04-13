export interface TableInfo {
  schema: string;
  name: string;
  display_name: string;
  row_count_estimate: number | null;
}

export interface ColumnInfo {
  name: string;
  display_name: string;
  data_type: string;
  display_type: string;
  is_nullable: boolean;
  is_primary_key: boolean;
}

export interface QueryResult {
  columns: ColumnInfo[];
  rows: Record<string, unknown>[];
  total_rows: number;
  page: number;
  page_size: number;
}

export type SortDirection = 'asc' | 'desc';

export interface SortState {
  column: string | null;
  direction: SortDirection | null;
}

export type FilterState = Record<string, string>;

export interface TablesResponse {
  tables: TableInfo[];
}

export interface ColumnsResponse {
  columns: ColumnInfo[];
}

export interface StatusResponse {
  mode: 'normal' | 'setup';
}

export interface DisplayConfig {
  branding: {
    title: string | null;
    subtitle: string | null;
  };
  // Keyed by qualified "schema.table" (including "public.table").
  tables: Record<
    string,
    {
      display_name: string;
      columns: Record<string, { display_name: string }>;
    }
  >;
}

// ── Setup Wizard Types ──────────────────────────────────────────────────────

export interface SshWizardConfig {
  host: string;
  port: number;
  username: string;
  auth_method: 'key' | 'password' | 'agent';
  key_path?: string;
  key_passphrase?: string; // only in memory, never persisted to seeki.toml
  password?: string; // ssh password auth (not yet implemented backend-side)
}

export interface TablePreview {
  schema: string;
  name: string; // always the bare table name (never "schema.table")
  estimated_rows: number;
  is_system: boolean;
}

export interface SchemaPreview {
  name: string;
  table_count: number;
}

export interface TestConnectionResult {
  success: boolean;
  tables?: TablePreview[];
  schemas?: SchemaPreview[];
  error?: string;
  error_source?: 'ssh' | 'db' | 'ssh_config';
}

export interface WizardData {
  // Step 1
  connection_mode: 'url' | 'fields';
  url: string;
  // fields mode
  host: string;
  port: number;
  database: string;
  db_user: string;
  db_password: string;
  // SSH
  use_ssh: boolean;
  ssh: SshWizardConfig;
  // Step 2
  tables: TablePreview[]; // from test-connection response
  schemas: SchemaPreview[]; // from test-connection response
  selected_tables: string[];
  selected_schemas: string[];
  // Step 3
  title: string;
  subtitle: string;
}

export interface SetupSaveRequest {
  server?: { host: string; port: number };
  database: {
    kind: string;
    url: string;
    max_connections: number;
    schemas?: string[];
  };
  ssh?: SshWizardConfig;
  tables?: { include: string[] };
  branding?: { title: string; subtitle?: string };
}

export interface SetupSaveResponse {
  success: boolean;
  error?: string;
}
