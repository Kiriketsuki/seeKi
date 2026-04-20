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

export type ViewAggregate = 'SUM' | 'AVG' | 'COUNT' | 'MIN' | 'MAX' | 'LATEST';

export type ViewTemplateId =
  | 'scratch'
  | 'most-recent-per-group'
  | 'counts-per-day'
  | 'top-n-per-group'
  | 'totals-by-week'
  | 'previous-row-delta';

export type ViewFilterValue =
  | {
      op:
        | 'eq'
        | 'gt'
        | 'gte'
        | 'lt'
        | 'lte'
        | 'contains'
        | 'starts_with';
      value: string;
    }
  | {
      op: 'between';
      value: [string, string];
    }
  | {
      op: 'is_empty';
    }
  | {
      op: 'in_list';
      value: string[];
    };

export type ViewDefinitionFilters = Record<string, ViewFilterValue>;

export type ViewSourceKind = 'fk' | 'match' | 'self';

export interface ViewSourceRef {
  id: string;
  kind: ViewSourceKind;
  schema: string;
  table: string;
  label?: string | null;
  match?: {
    base_column: string;
    source_column: string;
  } | null;
  self?: {
    entity_column: string;
    order_column: string;
    direction: 'previous' | 'next';
  } | null;
}

export interface ViewColumnRef {
  source_id?: string | null;
  source_schema: string;
  source_table: string;
  column_name: string;
}

export interface ViewOrderBy {
  source_id?: string | null;
  source_schema: string;
  source_table: string;
  column_name: string;
  direction: 'asc' | 'desc';
}

export interface ViewGrouping {
  keys: ViewColumnRef[];
  latest_by?: ViewOrderBy | null;
}

export interface ViewRanking {
  partition_by: ViewColumnRef[];
  order_by?: ViewOrderBy | null;
  limit: number;
}

export type ViewDerivedOperation =
  | 'difference'
  | 'ratio_percent'
  | 'age_of_timestamp'
  | 'date_bucket'
  | 'date_part'
  | 'text_concat'
  | 'text_length'
  | 'if_then';

export interface ViewDerivedInput {
  kind: 'column' | 'literal';
  source_id?: string | null;
  source_schema?: string | null;
  source_table?: string | null;
  column_name?: string | null;
  value?: string | null;
}

export interface ViewDerivedColumn {
  alias?: string | null;
  operation: ViewDerivedOperation;
  inputs: ViewDerivedInput[];
  options?: Record<string, unknown> | null;
}

export interface ViewColumn {
  kind?: 'source' | 'derived';
  source_id?: string | null;
  source_schema: string;
  source_table: string;
  column_name: string;
  alias?: string | null;
  aggregate?: ViewAggregate | null;
  derived?: ViewDerivedColumn | null;
}

export interface FkHop {
  from_schema: string;
  from_table: string;
  from_columns: string[];
  to_schema: string;
  to_table: string;
  to_columns: string[];
  constraint_name: string;
}

export interface SavedViewSummary {
  id: number;
  name: string;
  base_schema: string;
  base_table: string;
  definition_version: number;
  created_at: string;
  updated_at: string;
}

export interface ViewDefinitionShape {
  columns: ViewColumn[];
  filters: ViewDefinitionFilters;
  sources?: ViewSourceRef[];
  grouping?: ViewGrouping | null;
  ranking?: ViewRanking | null;
  template?: ViewTemplateId | null;
}

export interface SavedViewDefinition extends SavedViewSummary, ViewDefinitionShape {}

export interface ViewDraft extends ViewDefinitionShape {
  name: string;
  base_schema: string;
  base_table: string;
  definition_version?: number;
}

export type SortDirection = 'asc' | 'desc';

export interface SortEntry {
  column: string;
  direction: SortDirection;
}

export type SortState = SortEntry[];

export type FilterState = Record<string, string>;

export interface TablesResponse {
  tables: TableInfo[];
}

export interface ColumnsResponse {
  columns: ColumnInfo[];
}

export interface SavedViewsResponse {
  views: SavedViewSummary[];
}

export interface SavedViewResponse {
  view: SavedViewDefinition;
}

export interface FkPathResponse {
  path: FkHop[];
}

export interface ColumnSamplesResponse {
  samples: string[];
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

// ── Update Patcher Types ──────────────────────────────────────────────────

export interface VersionInfo {
  version: string;
  commit: string;
  built_at: string;
}

export type VersionResponse = VersionInfo;

export type UpdatePollIntervalHours = 0 | 1 | 6 | 24;

export interface UpdateStatus {
  current: string;
  latest: string | null;
  pre_release_channel: boolean;
  poll_interval_hours: UpdatePollIntervalHours;
  update_available: boolean;
  previous_exists: boolean;
  last_checked: string | null;
  release_notes: string | null;
  available_builds: AvailableBuild[];
}

export interface AvailableBuild {
  tag: string;
  published_at: string;
}

export interface WipUploadResult {
  upload_id: string;
  sha256: string;
  size: number;
}

export interface ApplyResult {
  status: string;
  message: string;
}

export interface RollbackResult {
  status: string;
  message: string;
}

// ── Preferences / Store Types ───────────────────────────────────────────────

export interface SortColumn {
  col: string;
  dir: SortDirection;
}

export interface SortPreset {
  id: number;
  name: string;
  columns: SortColumn[];
}

export interface FilterPreset {
  id: number;
  name: string;
  filters: FilterState;
}

export interface LastUsedTableState {
  sort_columns: SortColumn[];
  filters: FilterState;
  search_term: string | null;
}

export type SidebarMode = 'tables' | 'settings';

export type TablesSurface =
  | { kind: 'table' }
  | { kind: 'builder' }
  | { kind: 'view'; viewId: number };

export type SettingsSection =
  | 'updates'
  | 'branding'
  | 'appearance'
  | 'connection'
  | 'data'
  | 'about';

export type DateFormatPreference =
  | 'system'
  | 'YYYY-MM-DD'
  | 'DD/MM/YYYY'
  | 'MM/DD/YYYY';

export type RowDensityPreference = 'comfortable' | 'compact';

export interface BrandingSettings {
  title: string;
  subtitle: string;
}

export interface AppearanceSettings {
  dateFormat: DateFormatPreference;
  rowDensity: RowDensityPreference;
}

export type SettingsEntries = Record<string, unknown>;

export interface ConnectionStatusResponse {
  database_kind: 'postgres' | 'sqlite';
  host: string | null;
  port: number | null;
  database: string | null;
  schemas: string[];
  ssh_enabled: boolean;
  ssh_connected: boolean;
}
