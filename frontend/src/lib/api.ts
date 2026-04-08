import type {
  TableInfo,
  ColumnInfo,
  QueryResult,
  DisplayConfig,
  StatusResponse,
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

function assertShape(data: unknown, fields: string[], context: string): void {
  if (data == null || typeof data !== 'object') {
    throw new Error(`${context}: expected object, got ${typeof data}`);
  }
  for (const field of fields) {
    if (!(field in (data as Record<string, unknown>))) {
      throw new Error(`${context}: missing required field '${field}'`);
    }
  }
}

async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(path);
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    let message = `API error ${res.status}`;
    try {
      const body = JSON.parse(text);
      if (body?.error) message = body.error;
    } catch {
      if (text) message += `: ${text}`;
    }
    throw new Error(message);
  }
  return res.json() as Promise<T>;
}

export async function fetchTables(): Promise<TableInfo[]> {
  if (USE_MOCK) return mockFetchTables();
  const data = await apiFetch<TablesResponse>('/api/tables');
  assertShape(data, ['tables'], '/api/tables');
  return data.tables;
}

export async function fetchColumns(table: string): Promise<ColumnInfo[]> {
  if (USE_MOCK) return mockFetchColumns(table);
  const data = await apiFetch<ColumnsResponse>(
    `/api/tables/${encodeURIComponent(table)}/columns`,
  );
  assertShape(data, ['columns'], `/api/tables/${table}/columns`);
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
  const result = await apiFetch<QueryResult>(path);
  assertShape(result, ['rows', 'total_rows', 'page', 'page_size'], `/api/tables/${table}/rows`);
  return result;
}

export async function fetchDisplayConfig(): Promise<DisplayConfig> {
  if (USE_MOCK) return mockFetchDisplayConfig();
  const data = await apiFetch<DisplayConfig>('/api/config/display');
  assertShape(data, ['branding', 'tables'], '/api/config/display');
  return data;
}

export async function fetchStatus(): Promise<StatusResponse> {
  if (USE_MOCK) return { mode: 'normal' };
  const data = await apiFetch<StatusResponse>('/api/status');
  assertShape(data, ['mode'], '/api/status');
  return data;
}
