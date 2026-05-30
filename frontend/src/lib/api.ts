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
  WipUploadResult,
  ApplyResult,
  RollbackResult,
  SortPreset,
  FilterPreset,
  LastUsedTableState,
  UpdatePollIntervalHours,
  SavedViewSummary,
  SavedViewDefinition,
  ViewDraft,
  FkHop,
  ColumnSamplesResponse,
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
let updateTokenValue: string | null = null;
let updateTokenPromise: Promise<string> | null = null;

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
      const parsed = JSON.parse(text);
      if (parsed?.error) message = `API error ${res.status}: ${parsed.error}`;
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
      const parsed = JSON.parse(text);
      if (parsed?.error) message = `API error ${res.status}: ${parsed.error}`;
    } catch {
      if (text) message += `: ${text}`;
    }
    throw new Error(message);
  }
  if (res.status === 204) return undefined as unknown as T;
  return res.json() as Promise<T>;
}

async function apiPatch<T = void>(path: string, body: unknown): Promise<T> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetch(path, {
      method: 'PATCH',
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
      const parsed = JSON.parse(text);
      if (parsed?.error) message = `API error ${res.status}: ${parsed.error}`;
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

export interface ViewRowsParams extends FetchRowsParams {}

export async function fetchRows(
  schema: string,
  table: string,
  params?: FetchRowsParams,
): Promise<QueryResult> {
  if (USE_MOCK) return mockFetchRows(schema, table, params);
  const base = `/api/tables/${encodeURIComponent(schema)}/${encodeURIComponent(table)}/rows`;
  const path = `${base}${buildRowsQueryString(params)}`;
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

function buildRowsQueryString(params?: FetchRowsParams): string {
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
  return qs ? `?${qs}` : '';
}

export async function fetchViews(): Promise<SavedViewSummary[]> {
  if (USE_MOCK) return [];
  try {
    const data = await apiFetch<{ views: SavedViewSummary[] }>('/api/views');
    assertShape(data, ['views'], '/api/views');
    return data.views;
  } catch (error) {
    if (isApiStatusError(error, 404)) {
      return [];
    }
    throw error;
  }
}

export async function createView(draft: ViewDraft): Promise<SavedViewSummary> {
  if (USE_MOCK) {
    throw new Error('Saved views are not supported in mock mode');
  }
  const data = await apiPost<{ view: SavedViewSummary }>('/api/views', {
    name: draft.name,
    base_schema: draft.base_schema,
    base_table: draft.base_table,
    columns: draft.columns,
    filters: draft.filters,
    sources: draft.sources,
    grouping: draft.grouping,
    ranking: draft.ranking,
    template: draft.template,
  });
  assertShape(data, ['view'], '/api/views');
  return data.view;
}

export async function fetchView(viewId: number): Promise<SavedViewDefinition> {
  if (USE_MOCK) {
    throw new Error('Saved views are not supported in mock mode');
  }
  const path = `/api/views/${viewId}`;
  const data = await apiFetch<{ view: SavedViewDefinition }>(path);
  assertShape(data, ['view'], path);
  return data.view;
}

export async function renameView(
  viewId: number,
  name: string,
): Promise<SavedViewSummary | null> {
  if (USE_MOCK) {
    throw new Error('Saved views are not supported in mock mode');
  }
  const path = `/api/views/${viewId}`;
  const data = await apiPatch<{ view?: SavedViewSummary } | void>(path, { name });
  if (data && typeof data === 'object' && 'view' in data && data.view) {
    return data.view;
  }
  return null;
}

export async function deleteView(viewId: number): Promise<void> {
  if (USE_MOCK) return;
  await apiFetch(`/api/views/${viewId}`, 'DELETE');
}

export async function previewView(
  draft: Omit<ViewDraft, 'name'> | ViewDraft,
): Promise<QueryResult> {
  if (USE_MOCK) {
    return { columns: [], rows: [], total_rows: 0, page: 1, page_size: 100 };
  }
  const data = await apiPost<QueryResult>('/api/views/preview', {
    base_schema: draft.base_schema,
    base_table: draft.base_table,
    columns: draft.columns,
    filters: draft.filters,
    sources: draft.sources,
    grouping: draft.grouping,
    ranking: draft.ranking,
    template: draft.template,
  });
  assertShape(data, ['rows', 'total_rows', 'page', 'page_size'], '/api/views/preview');
  return data;
}

export async function fetchViewRows(
  viewId: number,
  params?: ViewRowsParams,
): Promise<QueryResult> {
  if (USE_MOCK) {
    return { columns: [], rows: [], total_rows: 0, page: 1, page_size: 50 };
  }
  const base = `/api/views/${viewId}/rows`;
  const result = await apiFetch<QueryResult>(`${base}${buildRowsQueryString(params)}`);
  assertShape(result, ['rows', 'total_rows', 'page', 'page_size'], base);
  return result;
}

export function exportViewCsv(viewId: number, params?: ViewRowsParams): void {
  if (USE_MOCK) return;
  window.open(buildViewCsvUrl(viewId, params), '_blank');
}

export function buildViewCsvUrl(
  viewId: number,
  params?: ViewRowsParams,
): string {
  return `/api/views/${viewId}/csv${buildRowsQueryString(params)}`;
}

export async function fetchFkPath(
  baseSchema: string,
  baseTable: string,
  targetSchema: string,
  targetTable: string,
): Promise<FkHop[]> {
  if (USE_MOCK) return [];
  const searchParams = new URLSearchParams({
    base_schema: baseSchema,
    base_table: baseTable,
    target_schema: targetSchema,
    target_table: targetTable,
  });
  const path = `/api/views/fk-path?${searchParams.toString()}`;
  try {
    const data = await apiFetch<{ path: FkHop[] }>(path);
    assertShape(data, ['path'], '/api/views/fk-path');
    return data.path;
  } catch (error) {
    if (isApiStatusError(error, 404)) {
      return [];
    }
    throw error;
  }
}

export async function fetchColumnSamples(
  schema: string,
  table: string,
  column: string,
): Promise<string[]> {
  if (USE_MOCK) return [];
  const searchParams = new URLSearchParams({ column });
  const path = `/api/tables/${encodeURIComponent(schema)}/${encodeURIComponent(table)}/samples?${searchParams.toString()}`;
  const data = await apiFetch<ColumnSamplesResponse>(path);
  assertShape(data, ['samples'], path);
  return data.samples;
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

export async function fetchUpdateToken(): Promise<string> {
  if (USE_MOCK) return 'mock-update-token';
  if (updateTokenValue) return updateTokenValue;

  if (!updateTokenPromise) {
    updateTokenPromise = apiFetch<{ token: string }>('/api/update/token')
      .then((data) => {
        assertShape(data, ['token'], '/api/update/token');
        updateTokenValue = data.token;
        return data.token;
      })
      .catch((error) => {
        updateTokenPromise = null;
        throw error;
      });
  }

  return updateTokenPromise;
}

function resetUpdateTokenCache(): void {
  updateTokenValue = null;
  updateTokenPromise = null;
}

function withBearerHeader(headers: HeadersInit | undefined, token: string): Headers {
  const merged = new Headers(headers);
  merged.set('Authorization', `Bearer ${token}`);
  return merged;
}

async function fetchWithUpdateAuth(path: string, init: RequestInit): Promise<Response> {
  let token = await fetchUpdateToken();
  let response = await fetch(path, {
    ...init,
    headers: withBearerHeader(init.headers, token),
  });

  if (response.status !== 401 || USE_MOCK) {
    return response;
  }

  resetUpdateTokenCache();
  token = await fetchUpdateToken();
  response = await fetch(path, {
    ...init,
    headers: withBearerHeader(init.headers, token),
  });
  return response;
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

const UPDATE_STATUS_FIELDS = [
  'current',
  'latest',
  'pre_release_channel',
  'poll_interval_hours',
  'update_available',
  'previous_exists',
  'last_checked',
  'release_notes',
  'available_builds',
];

export async function fetchUpdateStatus(): Promise<UpdateStatus | null> {
  if (USE_MOCK) return mockFetchUpdateStatus();
  try {
    const data = await apiFetch<UpdateStatus>('/api/update/status');
    assertShape(data, UPDATE_STATUS_FIELDS, '/api/update/status');
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
    assertShape(data, UPDATE_STATUS_FIELDS, '/api/update/check');
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

export async function clearAllPresets(): Promise<void> {
  await apiFetch('/api/preferences/presets', 'DELETE');
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

export async function applyUpdate(
  source: 'release' | 'wip',
  wipUploadId?: string,
  releaseTag?: string,
): Promise<ApplyResult> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), SETUP_TIMEOUT_MS);
  let res: Response;
  try {
    res = await fetchWithUpdateAuth('/api/update/apply', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ source, wip_upload_id: wipUploadId, release_tag: releaseTag }),
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
    res = await fetchWithUpdateAuth('/api/update/wip', {
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
    res = await fetchWithUpdateAuth('/api/update/rollback', {
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

export async function updateSettings(options: {
  preReleaseChannel?: boolean;
  pollIntervalHours?: UpdatePollIntervalHours;
}): Promise<UpdateStatus> {
  const payload: Record<string, unknown> = {};
  if (options.preReleaseChannel !== undefined) {
    payload.pre_release_channel = options.preReleaseChannel;
  }
  if (options.pollIntervalHours !== undefined) {
    payload.poll_interval_hours = options.pollIntervalHours;
  }

  const data = await apiPatch<UpdateStatus>('/api/update/settings', payload);
  assertShape(data, UPDATE_STATUS_FIELDS, '/api/update/settings');
  return data;
}
