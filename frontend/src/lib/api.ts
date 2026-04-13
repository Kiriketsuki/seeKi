import type {
  TableInfo,
  ColumnInfo,
  QueryResult,
  DisplayConfig,
  StatusResponse,
  TablesResponse,
  ColumnsResponse,
  SshWizardConfig,
  TestConnectionResult,
  SetupSaveRequest,
  SetupSaveResponse,
  VersionInfo,
  UpdateStatus,
  CheckResult,
  WipUploadResult,
  ApplyResult,
  RollbackResult,
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

const DEFAULT_TIMEOUT_MS = 30_000;
const SETUP_TIMEOUT_MS = 60_000;

async function apiFetch<T>(path: string): Promise<T> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch(path, { signal: controller.signal });
  } catch (e) {
    clearTimeout(timeout);
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error('Request timed out — the server may be busy. Try again.');
    }
    throw e;
  }
  clearTimeout(timeout);
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

export async function fetchColumns(
  schema: string,
  table: string,
): Promise<ColumnInfo[]> {
  if (USE_MOCK) return mockFetchColumns(schema, table);
  const path = `/api/tables/${encodeURIComponent(schema)}/${encodeURIComponent(table)}/columns`;
  const data = await apiFetch<ColumnsResponse>(path);
  assertShape(data, ['columns'], path);
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
  schema: string,
  table: string,
  params?: FetchRowsParams,
): Promise<QueryResult> {
  if (USE_MOCK) return mockFetchRows(schema, table, params);
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
  const base = `/api/tables/${encodeURIComponent(schema)}/${encodeURIComponent(table)}/rows`;
  const path = `${base}${qs ? `?${qs}` : ''}`;
  const result = await apiFetch<QueryResult>(path);
  assertShape(result, ['rows', 'total_rows', 'page', 'page_size'], base);
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

export async function getStatus(): Promise<StatusResponse> {
  return fetchStatus();
}

export async function setupTestConnection(req: {
  kind: string;
  url: string;
  ssh?: SshWizardConfig;
}): Promise<TestConnectionResult> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), SETUP_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch('/api/setup/test-connection', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
      signal: controller.signal,
    });
  } catch (e) {
    clearTimeout(timeout);
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error('Connection test timed out — SSH tunnel negotiation may be slow. Try again.');
    }
    throw e;
  }
  clearTimeout(timeout);
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    let message = `Setup test-connection failed (${res.status})`;
    try {
      const body = JSON.parse(text);
      if (body?.error) message = body.error;
    } catch {
      if (text) message += `: ${text}`;
    }
    throw new Error(message);
  }
  return res.json();
}

export async function setupSaveConfig(req: SetupSaveRequest): Promise<SetupSaveResponse> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), SETUP_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch('/api/setup/save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
      signal: controller.signal,
    });
  } catch (e) {
    clearTimeout(timeout);
    if (e instanceof DOMException && e.name === 'AbortError') {
      // Server may have completed while we timed out — check before erroring
      try {
        const status = await fetch('/api/status').then((r) => r.json());
        if ((status as { mode?: string })?.mode === 'normal') {
          window.location.reload();
          return undefined as unknown as SetupSaveResponse; // unreachable after reload
        }
      } catch { /* status check failed — fall through to timeout error */ }
      throw new Error('Config save timed out — SSH tunnel negotiation may be slow. Try again.');
    }
    throw e;
  }
  clearTimeout(timeout);
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    let message = `Setup save failed (${res.status})`;
    try {
      const body = JSON.parse(text);
      if (body?.error) message = body.error;
    } catch {
      if (text) message += `: ${text}`;
    }
    throw new Error(message);
  }
  return res.json();
}

// ── Update Patcher API ──────────────────────────────────────────────────

export async function fetchVersion(): Promise<VersionInfo> {
  return apiFetch<VersionInfo>('/api/version');
}

export async function fetchUpdateStatus(): Promise<UpdateStatus> {
  return apiFetch<UpdateStatus>('/api/update/status');
}

export async function checkForUpdate(): Promise<CheckResult> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), SETUP_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch('/api/update/check', {
      method: 'POST',
      signal: controller.signal,
    });
  } catch (e) {
    clearTimeout(timeout);
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error('Update check timed out.');
    }
    throw e;
  }
  clearTimeout(timeout);
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    let message = `Update check failed (${res.status})`;
    try { const body = JSON.parse(text); if (body?.error) message = body.error; } catch {}
    throw new Error(message);
  }
  return res.json();
}

export async function applyUpdate(source: 'release' | 'wip', wipUploadId?: string): Promise<ApplyResult> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), SETUP_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch('/api/update/apply', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ source, wip_upload_id: wipUploadId }),
      signal: controller.signal,
    });
  } catch (e) {
    clearTimeout(timeout);
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error('Apply timed out — the server may be restarting.');
    }
    throw e;
  }
  clearTimeout(timeout);
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    let message = `Apply failed (${res.status})`;
    try { const body = JSON.parse(text); if (body?.error) message = body.error; } catch {}
    throw new Error(message);
  }
  return res.json();
}

export async function uploadWipBinary(file: File): Promise<WipUploadResult> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), SETUP_TIMEOUT_MS);
  let res: Response;
  try {
    const arrayBuffer = await file.arrayBuffer();
    res = await fetch('/api/update/wip', {
      method: 'POST',
      body: arrayBuffer,
      signal: controller.signal,
    });
  } catch (e) {
    clearTimeout(timeout);
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error('Upload timed out.');
    }
    throw e;
  }
  clearTimeout(timeout);
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    let message = `Upload failed (${res.status})`;
    try { const body = JSON.parse(text); if (body?.error) message = body.error; } catch {}
    throw new Error(message);
  }
  return res.json();
}

export async function rollbackUpdate(): Promise<RollbackResult> {
  const res = await fetch('/api/update/rollback', { method: 'POST' });
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    let message = `Rollback failed (${res.status})`;
    try { const body = JSON.parse(text); if (body?.error) message = body.error; } catch {}
    throw new Error(message);
  }
  return res.json();
}

export async function updateSettings(preReleaseChannel: boolean): Promise<void> {
  const res = await fetch('/api/update/settings', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pre_release_channel: preReleaseChannel }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    let message = `Settings update failed (${res.status})`;
    try { const body = JSON.parse(text); if (body?.error) message = body.error; } catch {}
    throw new Error(message);
  }
}
