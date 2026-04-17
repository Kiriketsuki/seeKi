import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// Re-implement assertShape locally since it's not exported — tests validate the logic
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

describe('assertShape', () => {
  it('passes when all fields are present', () => {
    expect(() =>
      assertShape({ rows: [], total_rows: 0, page: 1, page_size: 50 }, ['rows', 'total_rows', 'page', 'page_size'], 'test')
    ).not.toThrow();
  });

  it('throws when a required field is missing', () => {
    expect(() =>
      assertShape({ rows: [] }, ['rows', 'total_rows'], 'test')
    ).toThrow("test: missing required field 'total_rows'");
  });

  it('throws when data is null', () => {
    expect(() =>
      assertShape(null, ['rows'], 'test')
    ).toThrow('test: expected object, got object');
  });

  it('throws when data is undefined', () => {
    expect(() =>
      assertShape(undefined, ['rows'], 'test')
    ).toThrow('test: expected object, got undefined');
  });

  it('throws when data is a primitive', () => {
    expect(() =>
      assertShape('hello', ['rows'], 'test')
    ).toThrow('test: expected object, got string');
  });

  it('passes with empty fields list', () => {
    expect(() => assertShape({}, [], 'test')).not.toThrow();
  });

  it('passes when fields exist with null values', () => {
    expect(() =>
      assertShape({ rows: null, total_rows: null }, ['rows', 'total_rows'], 'test')
    ).not.toThrow();
  });
});

// ── Helpers for update-API tests ─────────────────────────────────────────────

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function getAuthHeader(call: unknown[]): string | null {
  const init = call[1] as RequestInit | undefined;
  if (!init?.headers) return null;
  return new Headers(init.headers).get('Authorization');
}

function fullUpdateStatus() {
  return {
    current: '1.0.0',
    latest: '1.0.1',
    pre_release_channel: false,
    poll_interval_hours: 24,
    update_available: true,
    previous_exists: false,
    last_checked: null,
    release_notes: null,
  };
}

// ── Token caching (updateTokenValue / updateTokenPromise / fetchUpdateToken) ──

describe('fetchUpdateToken caching', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.resetModules();
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('VITE_MOCK', 'false');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it('fetches /api/update/token on first call and stores the token in cache', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ token: 'tok-1' }));
    const { fetchUpdateToken } = await import('./api');

    const token = await fetchUpdateToken();

    expect(token).toBe('tok-1');
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/update/token',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('returns cached value on subsequent calls without a second fetch', async () => {
    fetchMock.mockResolvedValue(jsonResponse({ token: 'cached-tok' }));
    const { fetchUpdateToken } = await import('./api');

    const first = await fetchUpdateToken();
    const second = await fetchUpdateToken();
    const third = await fetchUpdateToken();

    expect(first).toBe('cached-tok');
    expect(second).toBe('cached-tok');
    expect(third).toBe('cached-tok');
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('concurrent callers share the same in-flight promise (no duplicate fetches)', async () => {
    let resolver!: (r: Response) => void;
    fetchMock.mockImplementationOnce(
      () =>
        new Promise<Response>((resolve) => {
          resolver = resolve;
        }),
    );
    const { fetchUpdateToken } = await import('./api');

    const p1 = fetchUpdateToken();
    const p2 = fetchUpdateToken();
    const p3 = fetchUpdateToken();

    resolver(jsonResponse({ token: 'shared' }));

    const [t1, t2, t3] = await Promise.all([p1, p2, p3]);
    expect(t1).toBe('shared');
    expect(t2).toBe('shared');
    expect(t3).toBe('shared');
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('clears in-flight promise on failure so a later call can retry', async () => {
    fetchMock
      .mockResolvedValueOnce(new Response('boom', { status: 500 }))
      .mockResolvedValueOnce(jsonResponse({ token: 'after-retry' }));
    const { fetchUpdateToken } = await import('./api');

    await expect(fetchUpdateToken()).rejects.toThrow(/API error 500/);
    const token = await fetchUpdateToken();

    expect(token).toBe('after-retry');
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});

// ── Auth-header attachment (fetchWithUpdateAuth) ─────────────────────────────

describe('fetchWithUpdateAuth bearer-token attachment', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.resetModules();
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('VITE_MOCK', 'false');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it('attaches Authorization: Bearer <token> to /api/update/apply', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonResponse({ token: 'bearer-tok' }))
      .mockResolvedValueOnce(jsonResponse({ status: 'ok', message: '' }));
    const { applyUpdate } = await import('./api');

    await applyUpdate('release');

    expect(fetchMock).toHaveBeenCalledTimes(2);
    const [tokenCall, applyCall] = fetchMock.mock.calls;
    expect(tokenCall[0]).toBe('/api/update/token');
    expect(applyCall[0]).toBe('/api/update/apply');
    expect(getAuthHeader(applyCall)).toBe('Bearer bearer-tok');
  });

  it('attaches Authorization: Bearer <token> to /api/update/wip', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonResponse({ token: 'wip-tok' }))
      .mockResolvedValueOnce(jsonResponse({ upload_id: 'u', sha256: 's', size: 3 }));
    const { uploadWipBinary } = await import('./api');

    const file = new File([new Uint8Array([1, 2, 3])], 'wip.bin');
    await uploadWipBinary(file);

    expect(fetchMock).toHaveBeenCalledTimes(2);
    const [, wipCall] = fetchMock.mock.calls;
    expect(wipCall[0]).toBe('/api/update/wip');
    expect(getAuthHeader(wipCall)).toBe('Bearer wip-tok');
  });

  it('attaches Authorization: Bearer <token> to /api/update/rollback', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonResponse({ token: 'rb-tok' }))
      .mockResolvedValueOnce(jsonResponse({ status: 'ok', message: '' }));
    const { rollbackUpdate } = await import('./api');

    await rollbackUpdate();

    expect(fetchMock).toHaveBeenCalledTimes(2);
    const [, rbCall] = fetchMock.mock.calls;
    expect(rbCall[0]).toBe('/api/update/rollback');
    expect(getAuthHeader(rbCall)).toBe('Bearer rb-tok');
  });

  it('on 401 clears the token cache and retries with a fresh token', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonResponse({ token: 'stale' })) // 1. first token fetch
      .mockResolvedValueOnce(new Response('unauthorized', { status: 401 })) // 2. apply → 401
      .mockResolvedValueOnce(jsonResponse({ token: 'fresh' })) // 3. refreshed token
      .mockResolvedValueOnce(jsonResponse({ status: 'ok', message: 'done' })); // 4. apply → 200
    const { applyUpdate } = await import('./api');

    const result = await applyUpdate('release');

    expect(result).toEqual({ status: 'ok', message: 'done' });
    expect(fetchMock).toHaveBeenCalledTimes(4);

    const urls = fetchMock.mock.calls.map((c) => c[0]);
    expect(urls).toEqual([
      '/api/update/token',
      '/api/update/apply',
      '/api/update/token',
      '/api/update/apply',
    ]);

    const applyAuth = fetchMock.mock.calls
      .filter((c) => c[0] === '/api/update/apply')
      .map(getAuthHeader);
    expect(applyAuth).toEqual(['Bearer stale', 'Bearer fresh']);
  });

  it('on 401 for /api/update/rollback also refreshes and retries', async () => {
    fetchMock
      .mockResolvedValueOnce(jsonResponse({ token: 't1' }))
      .mockResolvedValueOnce(new Response('unauthorized', { status: 401 }))
      .mockResolvedValueOnce(jsonResponse({ token: 't2' }))
      .mockResolvedValueOnce(jsonResponse({ status: 'ok', message: '' }));
    const { rollbackUpdate } = await import('./api');

    await rollbackUpdate();

    const rollbackAuth = fetchMock.mock.calls
      .filter((c) => c[0] === '/api/update/rollback')
      .map(getAuthHeader);
    expect(rollbackAuth).toEqual(['Bearer t1', 'Bearer t2']);
  });
});

// ── UPDATE_STATUS_FIELDS agreement across status/check/settings ──────────────

const EXPECTED_UPDATE_STATUS_FIELDS = [
  'current',
  'latest',
  'pre_release_channel',
  'poll_interval_hours',
  'update_available',
  'previous_exists',
  'last_checked',
  'release_notes',
] as const;

describe('UPDATE_STATUS_FIELDS response-shape agreement', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.resetModules();
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('VITE_MOCK', 'false');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it('contract lists exactly 8 fields with expected names', () => {
    expect(EXPECTED_UPDATE_STATUS_FIELDS).toHaveLength(8);
    expect([...EXPECTED_UPDATE_STATUS_FIELDS].sort()).toEqual(
      [
        'current',
        'last_checked',
        'latest',
        'poll_interval_hours',
        'pre_release_channel',
        'previous_exists',
        'release_notes',
        'update_available',
      ].sort(),
    );
  });

  it('fetchUpdateStatus accepts a full UpdateStatus response', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(fullUpdateStatus()));
    const { fetchUpdateStatus } = await import('./api');

    const status = await fetchUpdateStatus();

    expect(status).toMatchObject(fullUpdateStatus());
  });

  it('checkForUpdates accepts a full UpdateStatus response', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(fullUpdateStatus()));
    const { checkForUpdates } = await import('./api');

    const status = await checkForUpdates();

    expect(status).toMatchObject(fullUpdateStatus());
  });

  it('updateSettings accepts a full UpdateStatus response', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(fullUpdateStatus()));
    const { updateSettings } = await import('./api');

    const status = await updateSettings({ preReleaseChannel: true });

    expect(status).toMatchObject(fullUpdateStatus());
  });

  it.each(EXPECTED_UPDATE_STATUS_FIELDS)(
    'fetchUpdateStatus rejects a response missing %s',
    async (field) => {
      const partial: Record<string, unknown> = fullUpdateStatus();
      delete partial[field];
      fetchMock.mockResolvedValueOnce(jsonResponse(partial));
      const { fetchUpdateStatus } = await import('./api');

      await expect(fetchUpdateStatus()).rejects.toThrow(
        `missing required field '${field}'`,
      );
    },
  );

  it.each(EXPECTED_UPDATE_STATUS_FIELDS)(
    'checkForUpdates rejects a response missing %s',
    async (field) => {
      const partial: Record<string, unknown> = fullUpdateStatus();
      delete partial[field];
      fetchMock.mockResolvedValueOnce(jsonResponse(partial));
      const { checkForUpdates } = await import('./api');

      await expect(checkForUpdates()).rejects.toThrow(
        `missing required field '${field}'`,
      );
    },
  );

  it.each(EXPECTED_UPDATE_STATUS_FIELDS)(
    'updateSettings rejects a response missing %s',
    async (field) => {
      const partial: Record<string, unknown> = fullUpdateStatus();
      delete partial[field];
      fetchMock.mockResolvedValueOnce(jsonResponse(partial));
      const { updateSettings } = await import('./api');

      await expect(updateSettings({ preReleaseChannel: false })).rejects.toThrow(
        `missing required field '${field}'`,
      );
    },
  );

  it('fetchUpdateStatus returns null on 404 (endpoint not yet live)', async () => {
    fetchMock.mockResolvedValueOnce(new Response('not found', { status: 404 }));
    const { fetchUpdateStatus } = await import('./api');

    expect(await fetchUpdateStatus()).toBeNull();
  });

  it('fetchUpdateStatus returns null on 503 (updater unavailable)', async () => {
    fetchMock.mockResolvedValueOnce(new Response('unavailable', { status: 503 }));
    const { fetchUpdateStatus } = await import('./api');

    expect(await fetchUpdateStatus()).toBeNull();
  });
});

describe('custom views API helpers', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.resetModules();
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('VITE_MOCK', 'false');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it('fetchViews returns an empty list when the endpoint is not available yet', async () => {
    fetchMock.mockResolvedValueOnce(new Response('not found', { status: 404 }));
    const { fetchViews } = await import('./api');

    await expect(fetchViews()).resolves.toEqual([]);
  });

  it('createView posts the agreed payload shape', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({
        view: {
          id: 7,
          name: 'Sales per customer',
          base_schema: 'public',
          base_table: 'orders',
          created_at: '2026-04-17T00:00:00Z',
          updated_at: '2026-04-17T00:00:00Z',
        },
      }),
    );
    const { createView } = await import('./api');

    await createView({
      name: 'Sales per customer',
      base_schema: 'public',
      base_table: 'orders',
      columns: [
        {
          source_schema: 'public',
          source_table: 'customers',
          column_name: 'name',
          alias: 'customer_name',
          aggregate: null,
        },
      ],
      filters: { customer_name: 'acme' },
    });

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/views',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          name: 'Sales per customer',
          base_schema: 'public',
          base_table: 'orders',
          columns: [
            {
              source_schema: 'public',
              source_table: 'customers',
              column_name: 'name',
              alias: 'customer_name',
              aggregate: null,
            },
          ],
          filters: { customer_name: 'acme' },
        }),
      }),
    );
  });

  it('fetchViewRows serializes sort, search, and filter params', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({
        columns: [],
        rows: [],
        total_rows: 0,
        page: 1,
        page_size: 50,
      }),
    );
    const { fetchViewRows } = await import('./api');

    await fetchViewRows(42, {
      page: 2,
      page_size: 25,
      sort: 'customer_name:asc',
      search: 'acme',
      filters: {
        customer_name: 'Acme',
        sum_orders__total: '100',
      },
    });

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/views/42/rows?page=2&page_size=25&sort=customer_name%3Aasc&search=acme&filter.customer_name=Acme&filter.sum_orders__total=100',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('buildViewCsvUrl keeps the existing query semantics for saved-view export', async () => {
    const { buildViewCsvUrl } = await import('./api');

    expect(
      buildViewCsvUrl(9, {
        sort: 'customer_name:desc',
        search: 'globex',
        filters: { customer_name: 'Globex' },
      }),
    ).toBe('/api/views/9/csv?sort=customer_name%3Adesc&search=globex&filter.customer_name=Globex');
  });

  it('fetchFkPath returns an empty path when the endpoint is not available yet', async () => {
    fetchMock.mockResolvedValueOnce(new Response('not found', { status: 404 }));
    const { fetchFkPath } = await import('./api');

    await expect(fetchFkPath('public', 'orders', 'public', 'customers')).resolves.toEqual([]);
  });
});
