import type { Page } from '@playwright/test';
import { test, expect, type SeekiHelpers } from './fixtures';

type TableSummary = {
  schema: string;
  name: string;
  display_name: string;
};

type ColumnSummary = {
  name: string;
  display_name: string;
  data_type: string;
  is_primary_key: boolean;
};

type TableCatalogEntry = {
  table: TableSummary;
  columns: ColumnSummary[];
};

const NUMERIC_TYPES = new Set([
  'smallint',
  'integer',
  'bigint',
  'numeric',
  'real',
  'double precision',
]);

const TEMPORAL_TYPES = new Set([
  'date',
  'timestamp',
  'timestamp without time zone',
  'timestamp with time zone',
  'timestamptz',
]);

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function uniqueViewName(prefix: string): string {
  return `${prefix} ${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function isNumericColumn(column: ColumnSummary): boolean {
  return NUMERIC_TYPES.has(column.data_type);
}

function isTemporalColumn(column: ColumnSummary): boolean {
  return TEMPORAL_TYPES.has(column.data_type);
}

function isEntityColumn(column: ColumnSummary): boolean {
  return column.name.endsWith('_id') && !column.is_primary_key;
}

function isBasicCandidateColumn(column: ColumnSummary): boolean {
  return !column.is_primary_key && column.name !== 'id';
}

function tableValue(table: TableSummary): string {
  return `${table.schema}.${table.name}`;
}

function tableLabel(allTables: TableSummary[], table: TableSummary): string {
  const collisions = allTables.filter((candidate) => candidate.name === table.name).length > 1;
  if (collisions || table.schema !== 'public') {
    return `${table.schema}.${table.name}`;
  }

  return table.name;
}

async function loadTableCatalog(page: Page): Promise<TableCatalogEntry[]> {
  const tablesResponse = await page.request.get('/api/tables');
  expect(tablesResponse.ok()).toBeTruthy();
  const tablesPayload = (await tablesResponse.json()) as { tables: TableSummary[] };

  const catalog = await Promise.all(
    tablesPayload.tables.map(async (table) => {
      const columnsResponse = await page.request.get(
        `/api/tables/${encodeURIComponent(table.schema)}/${encodeURIComponent(table.name)}/columns`,
      );
      expect(columnsResponse.ok()).toBeTruthy();
      const columnsPayload = (await columnsResponse.json()) as { columns: ColumnSummary[] };

      return {
        table,
        columns: columnsPayload.columns,
      };
    }),
  );

  return catalog;
}

function findScratchCandidate(catalog: TableCatalogEntry[]): {
  table: TableSummary;
  column: ColumnSummary;
} | null {
  for (const entry of catalog) {
    const column =
      entry.columns.find((candidate) => isBasicCandidateColumn(candidate) && !isTemporalColumn(candidate)) ??
      entry.columns.find(isBasicCandidateColumn);
    if (column) {
      return {
        table: entry.table,
        column,
      };
    }
  }

  return null;
}

function findAdvancedTemplateCandidate(catalog: TableCatalogEntry[]): {
  table: TableSummary;
  templateId: 'previous-row-delta' | 'most-recent-per-group';
} | null {
  for (const entry of catalog) {
    const hasEntity = entry.columns.some(isEntityColumn);
    const hasTimestamp = entry.columns.some(isTemporalColumn);
    const hasNumericMeasure = entry.columns.some(
      (candidate) => isNumericColumn(candidate) && !candidate.is_primary_key && !isEntityColumn(candidate),
    );

    if (hasEntity && hasTimestamp && hasNumericMeasure) {
      return {
        table: entry.table,
        templateId: 'previous-row-delta',
      };
    }
  }

  for (const entry of catalog) {
    const hasEntity = entry.columns.some(isEntityColumn);
    const hasTimestamp = entry.columns.some(isTemporalColumn);
    const hasValueColumn = entry.columns.some(
      (candidate) =>
        !candidate.is_primary_key &&
        candidate.name !== 'id' &&
        (isNumericColumn(candidate) || !isTemporalColumn(candidate)),
    );

    if (hasEntity && hasTimestamp && hasValueColumn) {
      return {
        table: entry.table,
        templateId: 'most-recent-per-group',
      };
    }
  }

  return null;
}

function findCountsPerDayCandidate(catalog: TableCatalogEntry[]): TableSummary | null {
  for (const entry of catalog) {
    if (entry.columns.some(isTemporalColumn)) return entry.table;
  }
  return null;
}

function findTopNPerGroupCandidate(catalog: TableCatalogEntry[]): TableSummary | null {
  for (const entry of catalog) {
    const hasEntity = entry.columns.some(isEntityColumn);
    const hasNumeric = entry.columns.some(
      (c) => isNumericColumn(c) && !c.is_primary_key && !isEntityColumn(c),
    );
    if (hasEntity && hasNumeric) return entry.table;
  }
  return null;
}

function findTotalsByWeekCandidate(catalog: TableCatalogEntry[]): TableSummary | null {
  for (const entry of catalog) {
    const hasTimestamp = entry.columns.some(isTemporalColumn);
    const hasNumeric = entry.columns.some(
      (c) => isNumericColumn(c) && !c.is_primary_key,
    );
    if (hasTimestamp && hasNumeric) return entry.table;
  }
  return null;
}

async function openTemplateGallery(page: Page, seeki: SeekiHelpers) {
  await seeki.openTables();
  const createButton = page.getByTestId('data-panel-create-view');
  if (!(await createButton.isVisible().catch(() => false))) {
    await seeki.toggleSidebar();
  }
  await expect(createButton).toBeVisible();
  await createButton.click();
  await expect(page.getByTestId('view-template-gallery')).toBeVisible();
}

async function selectBuilderBaseTable(
  page: Page,
  table: TableSummary,
) {
  await page
    .getByTestId('view-template-gallery')
    .getByLabel('Starting from')
    .selectOption(tableValue(table));
}

async function chooseTemplate(
  page: Page,
  templateId: string,
) {
  await page.getByTestId(`view-template-${templateId}`).click();
  await expect(page.getByTestId('view-builder-topbar')).toBeVisible();
}

async function addRawColumnFromScratch(
  page: Page,
  columnName: string,
) {
  await page.getByTestId('view-builder-add-slot').click();
  const picker = page.getByTestId('view-column-picker');
  await expect(picker).toBeVisible();

  await picker.locator('.picker__footer-right button.primary').click();

  const sourceColumnSelect = picker.getByLabel('Source column');
  await expect(sourceColumnSelect).toBeEnabled();
  await sourceColumnSelect.selectOption(columnName);

  await picker.locator('.picker__footer-right button.primary').click();
  await picker.locator('.picker__footer-right button.primary').click();
  await picker.getByTestId('view-column-picker-save').click();

  await expect(picker).not.toBeVisible();
}

async function saveViewFromBuilder(
  page: Page,
  name: string,
) {
  await page.getByTestId('view-builder-name').fill(name);
  await page.getByTestId('view-builder-save').click();
}

async function waitForViewInSidebar(
  seeki: SeekiHelpers,
  name: string,
) {
  await expect.poll(async () => await seeki.getSidebarViewNames()).toContain(name);
}

async function waitForViewToDisappear(
  seeki: SeekiHelpers,
  name: string,
) {
  await expect.poll(async () => await seeki.getSidebarViewNames()).not.toContain(name);
}

function viewButton(
  page: Page,
  name: string,
) {
  return page
    .getByTestId('data-panel-body-views')
    .getByRole('button', { name: new RegExp(`^${escapeRegex(name)}\\b`) })
    .first();
}

async function openSavedViewFromSidebar(
  page: Page,
  seeki: SeekiHelpers,
  name: string,
) {
  await viewButton(page, name).click();
  await seeki.waitForGridLoaded();
}

async function openTableFromSidebar(
  page: Page,
  seeki: SeekiHelpers,
  label: string,
) {
  await page
    .getByTestId('data-panel-body-tables')
    .getByRole('button', { name: new RegExp(`^${escapeRegex(label)}(?:\\s|$)`) })
    .click();
  await seeki.waitForGridLoaded();
}

async function createScratchView(
  page: Page,
  seeki: SeekiHelpers,
  candidate: { table: TableSummary; column: ColumnSummary },
  name: string,
) {
  await openTemplateGallery(page, seeki);
  await selectBuilderBaseTable(page, candidate.table);
  await chooseTemplate(page, 'scratch');
  await expect(page.getByTestId('view-builder-topbar')).toContainText('Start from scratch');

  await addRawColumnFromScratch(page, candidate.column.name);
  await expect(page.getByTestId('view-builder-grid-slots')).toContainText(candidate.column.name);

  await saveViewFromBuilder(page, name);
  await waitForViewInSidebar(seeki, name);
  await seeki.waitForGridLoaded();
  await expect(page.getByText('Read-only saved view')).toBeVisible();
}

test.describe.serial('Custom views — Infinite scroll', () => {
  test('saved view loads and shows data in current pagination mode', async ({ page, seeki }) => {
    const catalog = await loadTableCatalog(page);
    const candidate = findScratchCandidate(catalog);
    if (!candidate) {
      test.skip(true, 'No table with a usable source column available for this test.');
      return;
    }

    const viewName = uniqueViewName('E2E Infinite Scroll View');

    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    await createScratchView(page, seeki, candidate, viewName);

    // Status bar should be populated in whatever mode is active
    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(/(?:Showing \d[\d,]* - \d[\d,]* of|Loaded \d[\d,]* of) \d[\d,]*/);

    const total = await seeki.getTotalRows();
    expect(total).toBeGreaterThan(0);

    // Cleanup
    await page.getByRole('button', { name: 'Delete view' }).click();
    await waitForViewToDisappear(seeki, viewName);
  });

  test('saved view: scroll appends next batch in infinite mode', async ({ page, seeki }) => {
    const isInfinite = await (async () => {
      await page.goto('/');
      await seeki.waitForAppReady();
      await seeki.waitForGridLoaded();
      return seeki.isInfiniteMode();
    })();
    if (!isInfinite) {
      test.skip(true, 'Test requires infinite scroll mode');
      return;
    }

    const catalog = await loadTableCatalog(page);
    const candidate = findScratchCandidate(catalog);
    if (!candidate) {
      test.skip(true, 'No table with a usable source column available.');
      return;
    }

    const viewName = uniqueViewName('E2E View Scroll Append');

    await createScratchView(page, seeki, candidate, viewName);

    const initialLoaded = await seeki.getLoadedCount();
    const total = await seeki.getTotalRows();

    if (initialLoaded >= total) {
      // All rows loaded on first page — still passes (nothing more to scroll)
      expect(initialLoaded).toBeGreaterThan(0);
    } else {
      // Trigger scroll-to-bottom on the grid
      const rowsResponse = seeki.pendingGridRowsResponse();
      await page.locator('revo-grid').evaluate((el) => {
        const inner: Element | null =
          (el.shadowRoot as ShadowRoot | null)?.querySelector('[data-type="rgScrollable"]') ??
          (el.shadowRoot as ShadowRoot | null)?.querySelector('[class*="scroll"]') ??
          el;
        (inner as HTMLElement).scrollTop = (inner as HTMLElement).scrollHeight;
      });
      await rowsResponse;

      await page.waitForFunction(
        (prev) => {
          const el = document.querySelector('.statusbar .showing');
          const match = el?.textContent?.match(/Loaded\s+([\d,]+)\s+of/);
          return match ? parseInt(match[1].replace(/,/g, ''), 10) > prev : false;
        },
        initialLoaded,
        { timeout: 10_000 },
      );

      const newLoaded = await seeki.getLoadedCount();
      expect(newLoaded).toBeGreaterThan(initialLoaded);
    }

    // Cleanup
    await page.getByRole('button', { name: 'Delete view' }).click();
    await waitForViewToDisappear(seeki, viewName);
  });
});

test.describe.serial('Custom views', () => {
  test('create, reopen, and delete a saved view from the data panels scratch flow', async ({ page, seeki }) => {
    const catalog = await loadTableCatalog(page);
    const candidate = findScratchCandidate(catalog);
    if (!candidate) {
      test.skip(true, 'No table with a usable source column is available for scratch-builder coverage.');
      return;
    }

    const viewName = uniqueViewName('E2E Scratch View');

    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    await seeki.openTables();
    await expect(page.getByTestId('data-panel-header-tables')).toBeVisible();
    await expect(page.getByTestId('data-panel-header-views')).toBeVisible();

    await createScratchView(page, seeki, candidate, viewName);

    const firstTable = catalog[0];
    expect(firstTable).toBeTruthy();

    await openTableFromSidebar(page, seeki, tableLabel(catalog.map((entry) => entry.table), firstTable.table));
    await openSavedViewFromSidebar(page, seeki, viewName);

    await expect(page.getByText('Read-only saved view')).toBeVisible();
    await page.getByRole('button', { name: 'Delete view' }).click();

    await waitForViewToDisappear(seeki, viewName);
    await seeki.waitForGridLoaded();
  });

  test('delete failure surfaces the table error banner', async ({ page, seeki }) => {
    const catalog = await loadTableCatalog(page);
    const candidate = findScratchCandidate(catalog);
    if (!candidate) {
      test.skip(true, 'No table with a usable source column is available for delete-failure coverage.');
      return;
    }

    const viewName = uniqueViewName('E2E Delete Failure View');

    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    await createScratchView(page, seeki, candidate, viewName);

    await page.route('**/api/views/*', async (route) => {
      if (route.request().method() === 'DELETE') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'simulated server error' }),
        });
        return;
      }

      await route.continue();
    });

    await page.getByRole('button', { name: 'Delete view' }).click();

    await expect(page.getByText(/simulated server error/i)).toBeVisible();
    await waitForViewInSidebar(seeki, viewName);

    await page.unroute('**/api/views/*');
    await page.getByRole('button', { name: 'Dismiss' }).click();
    await page.getByRole('button', { name: 'Delete view' }).click();
    await waitForViewToDisappear(seeki, viewName);
  });

  test('builds and saves an advanced v2 template flow from the template gallery', async ({ page, seeki }) => {
    const catalog = await loadTableCatalog(page);
    const candidate = findAdvancedTemplateCandidate(catalog);
    if (!candidate) {
      test.skip(true, 'No table with the columns needed for an advanced template flow is available.');
      return;
    }

    const viewName = uniqueViewName('E2E Advanced Template View');

    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    await openTemplateGallery(page, seeki);
    await selectBuilderBaseTable(page, candidate.table);
    await chooseTemplate(page, candidate.templateId);

    if (candidate.templateId === 'previous-row-delta') {
      await expect(page.getByTestId('view-builder-topbar')).toContainText('Previous-row delta');
      await expect(page.getByTestId('view-builder-grid-slots')).toContainText(/delta/i);
    } else {
      await expect(page.getByTestId('view-builder-topbar')).toContainText('Most recent per group');
      await expect(page.getByTestId('view-builder-grouping')).not.toHaveValue('');
      await expect(page.getByTestId('view-builder-latest-by')).not.toHaveValue('');
    }

    await saveViewFromBuilder(page, viewName);
    await waitForViewInSidebar(seeki, viewName);
    await seeki.waitForGridLoaded();
    await expect(page.getByText('Read-only saved view')).toBeVisible();

    const headers = await seeki.getVisibleColumnHeaders();
    expect(headers.length).toBeGreaterThan(0);

    await page.getByRole('button', { name: 'Delete view' }).click();
    await waitForViewToDisappear(seeki, viewName);
  });

  test('counts-per-day template: creates a day-bucketed count view', async ({ page, seeki }) => {
    const catalog = await loadTableCatalog(page);
    const table = findCountsPerDayCandidate(catalog);
    if (!table) {
      test.skip(true, 'No table with a timestamp column available for counts-per-day.');
      return;
    }

    const viewName = uniqueViewName('E2E Counts Per Day');

    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    await openTemplateGallery(page, seeki);
    await selectBuilderBaseTable(page, table);
    await chooseTemplate(page, 'counts-per-day');

    await expect(page.getByTestId('view-builder-topbar')).toContainText('Counts per day');
    await expect(page.getByTestId('view-builder-grid-slots')).toContainText(/day/i);

    await saveViewFromBuilder(page, viewName);
    await waitForViewInSidebar(seeki, viewName);
    await seeki.waitForGridLoaded();
    await expect(page.getByText('Read-only saved view')).toBeVisible();

    const headers = await seeki.getVisibleColumnHeaders();
    expect(headers.length).toBeGreaterThan(0);

    await page.getByRole('button', { name: 'Delete view' }).click();
    await waitForViewToDisappear(seeki, viewName);
  });

  test('top-n-per-group template: creates a ranked view', async ({ page, seeki }) => {
    const catalog = await loadTableCatalog(page);
    const table = findTopNPerGroupCandidate(catalog);
    if (!table) {
      test.skip(true, 'No table with entity + numeric columns available for top-n-per-group.');
      return;
    }

    const viewName = uniqueViewName('E2E Top N Per Group');

    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    await openTemplateGallery(page, seeki);
    await selectBuilderBaseTable(page, table);
    await chooseTemplate(page, 'top-n-per-group');

    await expect(page.getByTestId('view-builder-topbar')).toContainText('Top N per group');
    await expect(page.getByTestId('view-builder-grid-slots')).toContainText(/.+/);

    await saveViewFromBuilder(page, viewName);
    await waitForViewInSidebar(seeki, viewName);
    await seeki.waitForGridLoaded();
    await expect(page.getByText('Read-only saved view')).toBeVisible();

    const headers = await seeki.getVisibleColumnHeaders();
    expect(headers.length).toBeGreaterThan(0);

    await page.getByRole('button', { name: 'Delete view' }).click();
    await waitForViewToDisappear(seeki, viewName);
  });

  test('totals-by-week template: creates a weekly-summed view', async ({ page, seeki }) => {
    const catalog = await loadTableCatalog(page);
    const table = findTotalsByWeekCandidate(catalog);
    if (!table) {
      test.skip(true, 'No table with timestamp + numeric columns available for totals-by-week.');
      return;
    }

    const viewName = uniqueViewName('E2E Totals By Week');

    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    await openTemplateGallery(page, seeki);
    await selectBuilderBaseTable(page, table);
    await chooseTemplate(page, 'totals-by-week');

    await expect(page.getByTestId('view-builder-topbar')).toContainText('Totals by week');
    await expect(page.getByTestId('view-builder-grid-slots')).toContainText(/week/i);

    await saveViewFromBuilder(page, viewName);
    await waitForViewInSidebar(seeki, viewName);
    await seeki.waitForGridLoaded();
    await expect(page.getByText('Read-only saved view')).toBeVisible();

    const headers = await seeki.getVisibleColumnHeaders();
    expect(headers.length).toBeGreaterThan(0);

    await page.getByRole('button', { name: 'Delete view' }).click();
    await waitForViewToDisappear(seeki, viewName);
  });
});
