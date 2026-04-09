export interface TableInfo {
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
  tables: Record<
    string,
    {
      display_name: string;
      columns: Record<string, { display_name: string }>;
    }
  >;
}
