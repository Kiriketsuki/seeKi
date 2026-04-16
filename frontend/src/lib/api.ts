import type {
  ConnectionStatusResponse,
  SettingsEntries,
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
  SortPreset,
  FilterPreset,
  LastUsedTableState,
  UpdateStatusResponse,
  VersionResponse,
} from './types';
import {
  mockFetchTables,
  mockFetchColumns,
  mockFetchRows,
  mockFetchConnectionStatus,
  mockFetchDisplayConfig,
  mockFetchSettings,
  mockFetchUpdateStatus,
  mockFetchVersion,
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

async function apiFetch<T = void>(path: string, method = 'GET'): Promise<T> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch(path, { method, signal: controller.signal });
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
  // 204 No Content — return undefined cast to T
  if (res.status === 204) return undefined as unknown as T;
  return res.json() as Promise<T>;
}

async function apiPost<T = void>(path: string, body: unknown): Promise<T> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: controller.signal,
    });
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
  if (res.status === 204) return undefined as unknown as T;
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
  sort?: string;
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
  if (params?.sort != null) searchParams.set('sort', params.sort);
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

export async function fetchConnectionStatus(): Promise<ConnectionStatusResponse> {
  if (USE_MOCK) return mockFetchConnectionStatus();
  const data = await apiFetch<ConnectionStatusResponse>('/api/connection-status');
  assertShape(
    data,
    ['database_kind', 'host', 'port', 'database', 'schemas', 'ssh_enabled', 'ssh_connected'],
    '/api/connection-status',
  );
  return data;
}

export async function fetchVersion(): Promise<VersionInfo> {
  if (USE_MOCK) return mockFetchVersion();
  const data = await apiFetch<VersionInfo>('/api/version');
  assertShape(data, ['version', 'commit', 'built_at'], '/api/version');
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

// ── Preferences API ───────────────────────────────────────────────────────────

export async function fetchSettings(): Promise<SettingsEntries> {
  if (USE_MOCK) return mockFetchSettings();
  return apiFetch<SettingsEntries>('/api/preferences/settings');
}

export async function saveSettings(entries: SettingsEntries): Promise<void> {
  await apiPost('/api/preferences/settings', entries);
}

function isApiStatusError(error: unknown, status: number): boolean {
  return error instanceof Error && error.message.startsWith(`API error ${status}`);
}

export async function fetchUpdateStatus(): Promise<UpdateStatus | null> {
  if (USE_MOCK) return mockFetchUpdateStatus();
  try {
    const data = await apiFetch<UpdateStatus>('/api/update/status');
    assertShape(
      data,
      ['current', 'latest', 'pre_release_channel', 'update_available', 'previous_exists', 'last_checked'],
      '/api/update/status',
    );
    return data;
  } catch (error) {
    if (isApiStatusError(error, 404) || isApiStatusError(error, 503)) {
      return null;
    }
    throw error;
  }
}

export async function checkForUpdates(): Promise<UpdateStatus | null> {
  try {
    const data = await apiPost<UpdateStatus>('/api/update/check', {});
    assertShape(
      data,
      ['current', 'latest', 'pre_release_channel', 'update_available', 'previous_exists', 'last_checked'],
      '/api/update/check',
    );
    return data;
  } catch (error) {
    if (isApiStatusError(error, 404) || isApiStatusError(error, 503)) {
      return null;
    }
    throw error;
  }
}

export async function fetchLastUsedState(
  schema: string,
  table: string,
): Promise<LastUsedTableState | null> {
  const path = `/api/preferences/presets/last-used/${encodeURIComponent(schema)}/${encodeURIComponent(table)}`;
  try {
    return await apiFetch<LastUsedTableState>(path);
  } catch (e) {
    // 404 means no saved state — return null rather than throwing
    if (e instanceof Error && e.message.startsWith('API error 404')) return null;
    throw e;
  }
}

export async function saveLastUsedState(
  schema: string,
  table: string,
  state: LastUsedTableState,
): Promise<void> {
  const path = `/api/preferences/presets/last-used/${encodeURIComponent(schema)}/${encodeURIComponent(table)}`;
  await apiPost(path, state);
}

export async function fetchSortPresets(schema: string, table: string): Promise<SortPreset[]> {
  const path = `/api/preferences/presets/sort/${encodeURIComponent(schema)}/${encodeURIComponent(table)}`;
  return apiFetch<SortPreset[]>(path);
}

export async function saveSortPreset(
  schema: string,
  table: string,
  name: string,
  columns: LastUsedTableState['sort_columns'],
): Promise<{ id: number }> {
  const path = `/api/preferences/presets/sort/${encodeURIComponent(schema)}/${encodeURIComponent(table)}`;
  return apiPost<{ id: number }>(path, { name, columns });
}

export async function deleteSortPreset(
  schema: string,
  table: string,
  name: string,
): Promise<void> {
  const path = `/api/preferences/presets/sort/${encodeURIComponent(schema)}/${encodeURIComponent(table)}/${encodeURIComponent(name)}`;
  await apiFetch(path, 'DELETE');
}

export async function fetchFilterPresets(schema: string, table: string): Promise<FilterPreset[]> {
  const path = `/api/preferences/presets/filter/${encodeURIComponent(schema)}/${encodeURIComponent(table)}`;
  return apiFetch<FilterPreset[]>(path);
}

export async function saveFilterPreset(
  schema: string,
  table: string,
  name: string,
  filters: Record<string, string>,
): Promise<{ id: number }> {
  const path = `/api/preferences/presets/filter/${encodeURIComponent(schema)}/${encodeURIComponent(table)}`;
  return apiPost<{ id: number }>(path, { name, filters });
}

export async function deleteFilterPreset(
  schema: string,
  table: string,
  name: string,
): Promise<void> {
  const path = `/api/preferences/presets/filter/${encodeURIComponent(schema)}/${encodeURIComponent(table)}/${encodeURIComponent(name)}`;
  await apiFetch(path, 'DELETE');
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
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), SETUP_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch('/api/update/rollback', {
      method: 'POST',
      signal: controller.signal,
    });
  } catch (e) {
    clearTimeout(timeout);
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error('Rollback timed out — the server may be restarting.');
    }
    throw e;
  }
  clearTimeout(timeout);
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
