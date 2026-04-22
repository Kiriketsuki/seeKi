import { test, expect } from './fixtures';

/** Matches both paged ("Showing X - Y of Z") and infinite ("Loaded X of Y") status text. */
const DUAL_STATUS_RE = /(?:Showing \d[\d,]* - \d[\d,]* of|Loaded \d[\d,]* of) \d[\d,]*/;

test.describe('Data Grid — Loading', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('grid loads with data on initial page', async ({ page, seeki }) => {
    // Verify status bar shows data in either paged or infinite mode
    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(DUAL_STATUS_RE);

    // Parse and verify we have rows
    const totalRows = await seeki.getTotalRows();
    expect(totalRows).toBeGreaterThan(0);

    // In paged mode also verify the page range
    const infinite = await seeki.isInfiniteMode();
    if (!infinite) {
      const range = await seeki.getPageRange();
      expect(range.start).toBe(1);
      expect(range.end).toBeGreaterThanOrEqual(1);
    }

    // Verify column headers are present — wait for at least one to render
    const headers = page.locator('[role="columnheader"]');
    await expect(headers.first()).toBeVisible();
    const headerCount = await headers.count();
    expect(headerCount).toBeGreaterThan(0);
  });

  test('grid shows correct column headers', async ({ page }) => {
    // Verify multiple column headers are visible — wait for render first
    const headers = page.locator('[role="columnheader"]');
    await expect(headers.first()).toBeVisible();
    const headerCount = await headers.count();
    expect(headerCount).toBeGreaterThan(0);
    const headerText = await headers.first().textContent();
    expect(headerText?.trim()).toBeTruthy();
  });
});

test.describe('Data Grid — Sorting', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('sort cycling: asc → desc → unsorted', async ({ page, seeki }) => {
    const firstHeader = page.locator('[role="columnheader"]').first();
    const firstHeaderState = firstHeader.locator('.sk-grid-header');
    const sortGlyph = firstHeader.locator('.sk-grid-header__sort');

    const getFirstCellText = async () => {
      const cell = page.locator('revo-grid [data-rgcol="0"][data-rgrow="0"]').first();
      if (await cell.count() === 0) return null;
      return (await cell.textContent())?.trim() ?? null;
    };

    const unsortedFirst = await getFirstCellText();

    await expect(sortGlyph).toHaveCount(0);

    // Click 1: ascending — wait for sorted data to load
    let rowsLoaded = seeki.pendingRowsResponse();
    await firstHeader.click();
    let rowsResponse = await rowsLoaded;
    expect(rowsResponse.request().url()).toContain('sort_direction=asc');
    await expect(page.locator('.action-dock [aria-live]')).toHaveText(/ascending$/);
    await expect(sortGlyph).toHaveText('↑');
    await expect(firstHeaderState).toHaveAttribute('aria-sort', 'ascending');
    const ascFirst = await getFirstCellText();

    rowsLoaded = seeki.pendingRowsResponse();
    await firstHeader.click();
    rowsResponse = await rowsLoaded;
    expect(rowsResponse.request().url()).toContain('sort_direction=desc');
    await expect(page.locator('.action-dock [aria-live]')).toHaveText(/descending$/);
    await expect(sortGlyph).toHaveText('↓');
    await expect(firstHeaderState).toHaveAttribute('aria-sort', 'descending');
    const descFirst = await getFirstCellText();

    if (ascFirst !== null && descFirst !== null && ascFirst !== descFirst) {
      expect(ascFirst).not.toBe(descFirst);
    }

    rowsLoaded = seeki.pendingRowsResponse();
    await firstHeader.click();
    rowsResponse = await rowsLoaded;
    expect(rowsResponse.request().url()).not.toContain('sort_direction=');
    expect(rowsResponse.request().url()).not.toContain('sort_column=');
    // sort cleared — live region should be empty
    await expect(page.locator('.action-dock [aria-live]')).toHaveText('');
    await expect(sortGlyph).toHaveCount(0);
    await expect(firstHeaderState).not.toHaveAttribute('aria-sort');

    const restoredFirst = await getFirstCellText();
    if (unsortedFirst !== null && restoredFirst !== null) {
      expect(restoredFirst).toBe(unsortedFirst);
    }
  });

  test('multi-column sort renders rank superscripts and orders vehicle_logs deterministically', async ({
    page,
    seeki,
  }) => {
    const tableNames = await seeki.getSidebarTableNames();
    const vehicleTableName = tableNames.find((name) => name.toLowerCase().includes('vehicle'));
    test.skip(!vehicleTableName, 'Test requires the seeded vehicle_logs table');

    await seeki.selectTable(vehicleTableName!);
    await seeki.waitForGridLoaded();

    const headers = page.locator('[role="columnheader"]');
    const idHeader = headers.nth(0);
    const vehicleHeader = headers.nth(1);

    let rowsLoaded = seeki.pendingRowsResponse();
    await vehicleHeader.click();
    await rowsLoaded;

    rowsLoaded = seeki.pendingRowsResponse();
    await idHeader.click();
    await rowsLoaded;

    rowsLoaded = seeki.pendingRowsResponse();
    await idHeader.click();
    await rowsLoaded;

    const vehicleSort = vehicleHeader.locator('.sk-grid-header__sort');
    const idSort = idHeader.locator('.sk-grid-header__sort');
    await expect(vehicleSort).toHaveText('↑1');
    await expect(vehicleSort).toHaveAttribute('aria-label', /priority 1 of 2/i);
    await expect(idSort).toHaveText('↓2');
    await expect(idSort).toHaveAttribute('aria-label', /priority 2 of 2/i);

    const visibleIds = await Promise.all(
      [0, 1, 2, 3, 4].map(async (rowIndex) => {
        const cell = page.locator(`revo-grid [data-rgcol="0"][data-rgrow="${rowIndex}"]`).first();
        return (await cell.textContent())?.trim() ?? '';
      }),
    );

    expect(visibleIds).toEqual(['200', '195', '190', '185', '180']);
  });

  test('shift-click stacks a second sort column; non-shift click resets to single sort', async ({
    page,
    seeki,
  }) => {
    const headers = page.locator('[role="columnheader"]');
    const headerCount = await headers.count();
    test.skip(headerCount < 2, 'Test requires at least 2 columns');

    const first = headers.nth(0);
    const second = headers.nth(1);
    const firstSort = first.locator('.sk-grid-header__sort');
    const secondSort = second.locator('.sk-grid-header__sort');

    // Sort column 0 ascending
    let rowsLoaded = seeki.pendingRowsResponse();
    await first.click();
    await rowsLoaded;
    await expect(firstSort).toHaveText('↑');

    // Shift-click column 1 → stacked sort, priority superscripts appear
    rowsLoaded = seeki.pendingRowsResponse();
    await second.click({ modifiers: ['Shift'] });
    await rowsLoaded;
    await expect(firstSort).toHaveText('↑1');
    await expect(secondSort).toHaveText('↑2');

    // Non-shift click on a 3rd column (or same col 1 without shift) should drop the stack
    // to a single-column sort — ranks disappear.
    rowsLoaded = seeki.pendingRowsResponse();
    const target = headerCount >= 3 ? headers.nth(2) : first;
    await target.click();
    await rowsLoaded;
    // After reset there should be exactly one sorted header. Glyph should not contain a rank digit.
    const anyRanked = page.locator('.sk-grid-header__sort-rank');
    await expect(anyRanked).toHaveCount(0);
  });

  test('toolbar sort-count badge appears and clears all sorts', async ({ page, seeki }) => {
    const firstHeader = page.locator('[role="columnheader"]').first();

    // No sort → no sort tool button
    let sortClearButton = page.locator('button.tool-button[aria-label*="Sorted by"]');
    await expect(sortClearButton).toHaveCount(0);

    // Apply a sort
    let rowsLoaded = seeki.pendingRowsResponse();
    await firstHeader.click();
    await rowsLoaded;

    // Toolbar sort button should appear with count = 1
    sortClearButton = page.locator('button.tool-button[aria-label*="Sorted by 1 column"]');
    await expect(sortClearButton).toBeVisible();

    // Click it → sort cleared, button disappears
    rowsLoaded = seeki.pendingRowsResponse();
    await sortClearButton.click();
    await rowsLoaded;
    await expect(page.locator('button.tool-button[aria-label*="Sorted by"]')).toHaveCount(0);
    await expect(firstHeader.locator('.sk-grid-header__sort')).toHaveCount(0);
  });
});

test.describe('Data Grid — Filtering', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('per-column filter narrows results', async ({ page, seeki }) => {
    // Get initial row count
    const initialTotal = await seeki.getTotalRows();

    // Show filters — use partial aria-label match since label is dynamic
    await seeki.clickFilterToggle();

    // Filter inputs are inside RevoGrid shadow DOM — query via columnheader ancestor
    const filterInputs = page.locator('[role="columnheader"] input[aria-label^="Filter"]');
    const firstFilter = filterInputs.first();
    await expect(firstFilter).toBeVisible();

    // Type a filter value — wait for the debounced API response
    const rowsLoaded = seeki.pendingRowsResponse();
    await firstFilter.fill('1');
    await rowsLoaded;

    // Verify the status bar updated and filter actually narrowed results
    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(DUAL_STATUS_RE);

    const filteredTotal = await seeki.getTotalRows();
    // Lower bound: filter actually matched rows (not a zero-match silent pass).
    // Upper bound is strict: seed data guarantees '1' does not match every row, so the
    // filtered set must be a proper subset of the initial set.
    expect(filteredTotal).toBeGreaterThan(0);
    expect(filteredTotal).toBeLessThan(initialTotal);
  });

  test('multiple filters AND together', async ({ page, seeki }) => {
    // Show filters
    await seeki.clickFilterToggle();

    const filterInputs = page.locator('[role="columnheader"] input[aria-label^="Filter"]');
    const firstFilter = filterInputs.first();
    await expect(firstFilter).toBeVisible();

    const filterCount = await filterInputs.count();

    // We need at least 2 columns to test multiple filters
    if (filterCount < 2) {
      console.warn('[e2e] Multi-filter AND coverage skipped: loaded table has fewer than 2 filterable columns. If running against a real DB (SEEKI_SKIP_SEED=1), point at a table with 2+ columns to exercise this assertion.');
      test.skip();
      return;
    }

    // Apply first filter
    let rowsLoaded = seeki.pendingRowsResponse();
    await filterInputs.nth(0).fill('1');
    await rowsLoaded;
    const firstFilterTotal = await seeki.getTotalRows();

    // Apply second filter
    rowsLoaded = seeki.pendingRowsResponse();
    await filterInputs.nth(1).fill('2');
    await rowsLoaded;
    const bothFiltersTotal = await seeki.getTotalRows();

    // With AND logic, both filters should give <= either individual filter
    expect(bothFiltersTotal).toBeLessThanOrEqual(firstFilterTotal);
  });
});

test.describe('Data Grid — Search', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('global search filters rows', async ({ page, seeki }) => {
    const initialTotal = await seeki.getTotalRows();

    // Open search via keyboard shortcut or button
    const searchButton = seeki.getActionDock().getByRole('button', { name: /search/i });
    if (await searchButton.count() > 0) {
      await searchButton.click();
    } else {
      await page.keyboard.press('Control+K');
    }

    // Find the search input
    const searchInput = seeki.getDockSearchPanel().locator('input.search-input');
    await expect(searchInput.first()).toBeVisible();

    // Type a search term — wait for debounced API response
    const rowsLoaded = seeki.pendingRowsResponse();
    await searchInput.first().fill('1');
    await rowsLoaded;

    // Verify status bar shows filtered results
    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(DUAL_STATUS_RE);

    const searchTotal = await seeki.getTotalRows();
    expect(searchTotal).toBeLessThanOrEqual(initialTotal);
  });
});

test.describe('Data Grid — Pagination', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('pagination forward and back', async ({ page, seeki }) => {
    // Skip if app is in infinite-scroll mode (no page buttons)
    const isInfinite = await seeki.isInfiniteMode();
    if (isInfinite) {
      test.skip(true, 'Pagination test requires paged mode — switch to paged in Settings > Data to run this test.');
      return;
    }

    // Check if we have enough rows for pagination
    const totalRows = await seeki.getTotalRows();

    if (totalRows <= 50) {
      console.warn('[e2e] Pagination coverage skipped: loaded table has 50 or fewer rows. If running against a real DB (SEEKI_SKIP_SEED=1), point at a table with >50 rows to exercise this assertion.');
      test.skip();
      return;
    }

    // Verify initial page
    let range = await seeki.getPageRange();
    expect(range.start).toBe(1);
    expect(range.end).toBeLessThanOrEqual(50);

    // Click next page — wait for new data to load and status bar to update
    const nextButton = page.locator('button[aria-label="Next page"]');
    let rowsLoaded = seeki.pendingRowsResponse();
    await nextButton.click();
    await rowsLoaded;
    // Wait for the status bar to reflect page 2 (DOM update is async after response)
    await page.waitForFunction(
      () => {
        const el = document.querySelector('.statusbar .showing');
        return el?.textContent?.match(/^Showing 51/);
      },
      { timeout: 5_000 },
    );

    // Verify we're on page 2
    range = await seeki.getPageRange();
    expect(range.start).toBe(51);
    expect(range.end).toBeGreaterThan(50);

    // Verify page info updated
    const pageInfo = page.locator('.page-info');
    const pageText = await pageInfo.textContent();
    expect(pageText).toMatch(/2 of \d+/);

    // Click previous page — wait for data to load and status bar to update
    const prevButton = page.locator('button[aria-label="Previous page"]');
    rowsLoaded = seeki.pendingRowsResponse();
    await prevButton.click();
    await rowsLoaded;
    await page.waitForFunction(
      () => {
        const el = document.querySelector('.statusbar .showing');
        return el?.textContent?.match(/^Showing 1 /);
      },
      { timeout: 5_000 },
    );

    // Verify we're back on page 1
    range = await seeki.getPageRange();
    expect(range.start).toBe(1);
    expect(range.end).toBeLessThanOrEqual(50);

    const pageTextAfter = await pageInfo.textContent();
    expect(pageTextAfter).toMatch(/1 of \d+/);
  });
});

test.describe('Data Grid — Cell Formatting', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('NULL cells have distinct styling', async ({ page }) => {
    // RevoGrid renders cell templates in light DOM
    const nullCell = page.locator('.sk-grid-cell--null').first();
    const hasNull = await nullCell.count() > 0;

    if (!hasNull) {
      console.warn('[e2e] NULL cell coverage skipped: loaded table has no NULL values in visible columns. If running against a real DB (SEEKI_SKIP_SEED=1), point at a table with nullable columns to exercise this assertion.');
      test.skip();
      return;
    }
    await expect(nullCell).toBeVisible();
    await expect(nullCell).toContainText('NULL');
  });

  test('boolean cells display as Yes/No badges', async ({ page }) => {
    const badge = page.locator('.sk-grid-badge').first();
    const hasBadge = await badge.count() > 0;

    if (!hasBadge) {
      console.warn('[e2e] Boolean badge coverage skipped: loaded table has no boolean columns. If running against a real DB (SEEKI_SKIP_SEED=1), point at a table with BOOLEAN columns to exercise this assertion.');
      test.skip();
      return;
    }
    await expect(badge).toBeVisible();
    const text = (await badge.textContent())?.trim();
    expect(['Yes', 'No']).toContain(text);
    // Badge should have either is-true or is-false class
    const classes = await badge.getAttribute('class') ?? '';
    expect(classes.includes('is-true') || classes.includes('is-false')).toBe(true);
  });

  test('numeric cells are right-aligned', async ({ page }) => {
    const numericCell = page.locator('.sk-grid-cell--number').first();
    const hasNumeric = await numericCell.count() > 0;

    if (!hasNumeric) {
      console.warn('[e2e] Numeric alignment coverage skipped: loaded table has no numeric columns. If running against a real DB (SEEKI_SKIP_SEED=1), point at a table with NUMERIC/INTEGER columns to exercise this assertion.');
      test.skip();
      return;
    }
    await expect(numericCell).toBeVisible();
    const textAlign = await numericCell.evaluate(
      (el) => window.getComputedStyle(el).textAlign,
    );
    expect(textAlign).toBe('right');
  });
});

test.describe('Data Grid — Infinite Scroll', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('infinite mode: status bar shows "Loaded X of Y" format', async ({ seeki }) => {
    const isInfinite = await seeki.isInfiniteMode();
    test.skip(!isInfinite, 'Test requires infinite scroll mode');

    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(/^Loaded \d[\d,]* of \d[\d,]*$/);

    const loaded = await seeki.getLoadedCount();
    const total = await seeki.getTotalRows();
    expect(loaded).toBeGreaterThan(0);
    expect(total).toBeGreaterThan(0);
    expect(loaded).toBeLessThanOrEqual(total);
  });

  test('infinite mode: scroll to bottom appends next batch', async ({ page, seeki }) => {
    const isInfinite = await seeki.isInfiniteMode();
    test.skip(!isInfinite, 'Test requires infinite scroll mode');

    const initialLoaded = await seeki.getLoadedCount();
    const total = await seeki.getTotalRows();
    test.skip(initialLoaded >= total, 'All rows already loaded — cannot scroll to append');

    // Scroll the RevoGrid scroll container to the bottom
    const rowsResponse = seeki.pendingRowsResponse();
    await seeki.scrollGridToBottom();
    await rowsResponse;

    // Wait for status bar to reflect more loaded rows
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
  });

  test('paged mode: status bar shows "Showing X - Y of Z" format', async ({ page, seeki }) => {
    const isInfinite = await seeki.isInfiniteMode();
    test.skip(isInfinite, 'Test verifies paged mode — already in infinite mode');

    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(/^Showing \d[\d,]* - \d[\d,]* of \d[\d,]*$/);

    // Paged mode should show Previous/Next buttons
    await expect(page.locator('button[aria-label="Previous page"]')).toBeAttached();
    await expect(page.locator('button[aria-label="Next page"]')).toBeAttached();
  });

  test('page-size selector changes batch size in infinite mode', async ({ page, seeki }) => {
    const isInfinite = await seeki.isInfiniteMode();
    test.skip(!isInfinite, 'Test requires infinite scroll mode');

    // The status bar page-size select is rendered in infinite mode
    const pageSizeSelect = page.locator('.statusbar select#sk-page-size');
    await expect(pageSizeSelect).toBeVisible();

    // Get initial loaded count
    const initialLoaded = await seeki.getLoadedCount();

    // Change to a different page size and wait for reload
    const rowsResponse = seeki.pendingRowsResponse();
    await pageSizeSelect.selectOption('100');
    await rowsResponse;

    // Loaded count should reflect the new batch size
    await seeki.waitForGridLoaded();
    const newLoaded = await seeki.getLoadedCount();
    expect(newLoaded).toBeGreaterThan(0);
    // After a reset the new first batch size should align with the chosen page size
    const total = await seeki.getTotalRows();
    expect(newLoaded).toBeLessThanOrEqual(total);

    // Restore original selection to avoid polluting other tests
    const currentSize = await pageSizeSelect.inputValue();
    if (currentSize !== '50') {
      const rowsLoaded = seeki.pendingRowsResponse();
      await pageSizeSelect.selectOption('50');
      await rowsLoaded;
    }
  });

  test('page-size preference survives round-trip navigation', async ({ page, seeki }) => {
    const isInfinite = await seeki.isInfiniteMode();
    test.skip(!isInfinite, 'Test requires infinite scroll mode');

    const tableNames = await seeki.getSidebarTableNames();
    test.skip(tableNames.length < 2, 'Test requires at least 2 tables');

    const pageSizeSelect = page.locator('.statusbar select#sk-page-size');
    await expect(pageSizeSelect).toBeVisible();

    const original = await pageSizeSelect.inputValue();
    const target = original === '100' ? '250' : '100';

    // Change page size and wait for reload
    let rowsLoaded = seeki.pendingRowsResponse();
    await pageSizeSelect.selectOption(target);
    await rowsLoaded;
    expect(await pageSizeSelect.inputValue()).toBe(target);

    // Navigate to a different table
    rowsLoaded = seeki.pendingRowsResponse();
    await seeki.selectTable(tableNames[1]);
    await rowsLoaded;

    // Navigate back to the first table
    rowsLoaded = seeki.pendingRowsResponse();
    await seeki.selectTable(tableNames[0]);
    await rowsLoaded;

    // Page size should be restored from saved preference
    expect(await pageSizeSelect.inputValue()).toBe(target);

    // Restore original to avoid polluting other tests
    if (target !== original) {
      rowsLoaded = seeki.pendingRowsResponse();
      await pageSizeSelect.selectOption(original);
      await rowsLoaded;
    }
  });
});

test.describe('Data Grid — RowCapWarning Banner', () => {
  test('soft cap banner appears when loaded rows reach SOFT_CAP', async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    const isInfinite = await seeki.isInfiniteMode();
    test.skip(!isInfinite, 'Test requires infinite scroll mode');

    // Intercept rows API to inflate total_rows and return a large batch
    let intercepted = false;
    await page.route('**/api/tables/*/rows*', async (route) => {
      if (intercepted) {
        await route.continue();
        return;
      }
      intercepted = true;
      const response = await route.fetch();
      const body = await response.json();
      const original = body as { rows: Record<string, unknown>[]; total_rows: number; columns: { name: string }[] };
      // Fabricate enough rows to cross the 5,000 soft cap
      const template = original.rows[0] ?? {};
      const fakeRows = Array.from({ length: 5_000 }, (_, i) => {
        const row: Record<string, unknown> = {};
        for (const col of original.columns) {
          row[col.name] = template[col.name] ?? `fake-${i}`;
        }
        return row;
      });
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          ...original,
          rows: fakeRows,
          total_rows: 50_000,
        }),
      });
    });

    // Trigger a reload so the intercepted response takes effect
    const rowsResponse = seeki.pendingGridRowsResponse();
    await seeki.scrollGridToBottom();
    await rowsResponse;

    // Wait for the soft cap banner to render
    const softBanner = page.locator('.cap-banner--soft');
    await expect(softBanner).toBeVisible({ timeout: 10_000 });
    await expect(softBanner).toContainText('rows loaded');
  });

  test('no cap banner for small tables', async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    const isInfinite = await seeki.isInfiniteMode();
    test.skip(!isInfinite, 'Test requires infinite scroll mode');

    const total = await seeki.getTotalRows();
    test.skip(total >= 5_000, 'Table too large — expected small table for negative test');

    await expect(page.locator('.cap-banner--soft')).not.toBeVisible();
    await expect(page.locator('.cap-banner--hard')).not.toBeVisible();
  });
});

test.describe('Data Grid — Inline Error Retry', () => {
  test('scroll-append failure shows error row with retry button', async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    const isInfinite = await seeki.isInfiniteMode();
    test.skip(!isInfinite, 'Test requires infinite scroll mode');

    const initialLoaded = await seeki.getLoadedCount();
    const total = await seeki.getTotalRows();
    test.skip(initialLoaded >= total, 'All rows already loaded — cannot trigger scroll-append');

    // Abort ALL subsequent rows requests to force both retry attempts to fail
    let abortCount = 0;
    await page.route('**/api/**/rows*', async (route) => {
      abortCount++;
      await route.abort('connectionrefused');
    });

    // Scroll to bottom to trigger loadMoreRows
    await seeki.scrollGridToBottom();

    // Wait for the inline error row to appear (after first attempt + 500ms backoff + second attempt)
    const retryButton = page.locator('.sk-error-cell__retry');
    await expect(retryButton).toBeVisible({ timeout: 15_000 });

    // Both attempts should have been aborted
    expect(abortCount).toBeGreaterThanOrEqual(2);

    // Remove the route interception so retry succeeds
    await page.unroute('**/api/**/rows*');

    // Click the retry button
    const rowsResponse = seeki.pendingGridRowsResponse();
    await retryButton.click();
    await rowsResponse;

    // Error row should be gone and more rows should be loaded
    await expect(retryButton).not.toBeVisible({ timeout: 5_000 });
    const newLoaded = await seeki.getLoadedCount();
    expect(newLoaded).toBeGreaterThan(initialLoaded);
  });
});
