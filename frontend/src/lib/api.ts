import type {
  TableInfo,
  ColumnInfo,
  QueryResult,
  DisplayConfig,
  TablesResponse,
  ColumnsResponse,
} from './types';
import {
  mockFetchTables,
  mockFetchColumns,
  mockFetchRows,
  mockFetchDisplayConfig,
} from './mock';

const USE_MOCK = import.meta.env.VITE_MOCK === 'true';

async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(path);
  if (!res.ok) {
    throw new Error(`API error ${res.status}: ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

export async function fetchTables(): Promise<TableInfo[]> {
  if (USE_MOCK) return mockFetchTables();
  const data = await apiFetch<TablesResponse>('/api/tables');
  return data.tables;
}

export async function fetchColumns(table: string): Promise<ColumnInfo[]> {
  if (USE_MOCK) return mockFetchColumns(table);
  const data = await apiFetch<ColumnsResponse>(
    `/api/tables/${encodeURIComponent(table)}/columns`,
  );
  return data.columns;
}

export interface FetchRowsParams {
  page?: number;
  page_size?: number;
  sort_column?: string;
  sort_direction?: string;
  search?: string;
  filters?: Record<string, string>;
}

export async function fetchRows(
  table: string,
  params?: FetchRowsParams,
): Promise<QueryResult> {
  if (USE_MOCK) return mockFetchRows(table, params);
  const searchParams = new URLSearchParams();
  if (params?.page != null) searchParams.set('page', String(params.page));
  if (params?.page_size != null)
    searchParams.set('page_size', String(params.page_size));
  if (params?.sort_column)
    searchParams.set('sort_column', params.sort_column);
  if (params?.sort_direction)
    searchParams.set('sort_direction', params.sort_direction);
  if (params?.search) searchParams.set('search', params.search);
  if (params?.filters) {
    for (const [col, val] of Object.entries(params.filters)) {
      searchParams.set(`filter.${col}`, val);
    }
  }
  const qs = searchParams.toString();
  const path = `/api/tables/${encodeURIComponent(table)}/rows${qs ? `?${qs}` : ''}`;
  return await apiFetch<QueryResult>(path);
}

export async function fetchDisplayConfig(): Promise<DisplayConfig> {
  if (USE_MOCK) return mockFetchDisplayConfig();
  return await apiFetch<DisplayConfig>('/api/config/display');
}
