import { test, expect } from './fixtures';

test.describe('Data Grid — Loading', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('grid loads with data on initial page', async ({ page, seeki }) => {
    // Verify status bar shows the expected range
    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(/Showing 1 - \d+ of \d+/);

    // Parse and verify we have rows
    const totalRows = await seeki.getTotalRows();
    expect(totalRows).toBeGreaterThan(0);

    const range = await seeki.getPageRange();
    expect(range.start).toBe(1);
    expect(range.end).toBeGreaterThanOrEqual(1);

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
    // Get the first sortable column header (via ARIA role)
    const firstHeader = page.locator('[role="columnheader"]').first();
    // The toolbar sort indicator reflects sort state in the light DOM
    const sortIndicator = page.locator('.tool-indicator');

    // Initial state: no sort
    await expect(sortIndicator).toHaveAttribute('aria-label', 'No active sort');

    // Click 1: ascending — wait for sorted data to load
    let rowsLoaded = seeki.pendingRowsResponse();
    await firstHeader.click();
    await rowsLoaded;
    await expect(sortIndicator).toHaveAttribute('aria-label', / asc$/);

    // Click 2: descending — wait for sorted data to load
    rowsLoaded = seeki.pendingRowsResponse();
    await firstHeader.click();
    await rowsLoaded;
    await expect(sortIndicator).toHaveAttribute('aria-label', / desc$/);

    // Click 3: back to unsorted — wait for data to reload
    rowsLoaded = seeki.pendingRowsResponse();
    await firstHeader.click();
    await rowsLoaded;
    await expect(sortIndicator).toHaveAttribute('aria-label', 'No active sort');
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
    const filterButton = page.locator('button.tool-button[aria-label*="ilters"]');
    await filterButton.click();

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
    expect(statusText).toMatch(/Showing \d+ - \d+ of \d+/);

    const filteredTotal = await seeki.getTotalRows();
    expect(filteredTotal).toBeLessThanOrEqual(initialTotal);
  });

  test('multiple filters AND together', async ({ page, seeki }) => {
    // Show filters
    const filterButton = page.locator('button.tool-button[aria-label*="ilters"]');
    await filterButton.click();

    const filterInputs = page.locator('[role="columnheader"] input[aria-label^="Filter"]');
    const firstFilter = filterInputs.first();
    await expect(firstFilter).toBeVisible();

    const filterCount = await filterInputs.count();

    // We need at least 2 columns to test multiple filters
    if (filterCount < 2) {
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
    const searchButton = page.locator('button.tool-button[aria-label*="earch"]').first();
    if (await searchButton.count() > 0) {
      await searchButton.click();
    } else {
      await page.keyboard.press('Control+K');
    }

    // Find the search input
    const searchInput = page.locator('input.search-input');
    await expect(searchInput.first()).toBeVisible();

    // Type a search term — wait for debounced API response
    const rowsLoaded = seeki.pendingRowsResponse();
    await searchInput.first().fill('1');
    await rowsLoaded;

    // Verify status bar shows filtered results
    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(/Showing \d+ - \d+ of \d+/);

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
    // Check if we have enough rows for pagination
    const totalRows = await seeki.getTotalRows();

    if (totalRows <= 50) {
      test.skip();
      return;
    }

    // Verify initial page
    let range = await seeki.getPageRange();
    expect(range.start).toBe(1);
    expect(range.end).toBeLessThanOrEqual(50);

    // Click next page — wait for new data to load
    const nextButton = page.locator('button[aria-label="Next page"]');
    let rowsLoaded = seeki.pendingRowsResponse();
    await nextButton.click();
    await rowsLoaded;

    // Verify we're on page 2
    range = await seeki.getPageRange();
    expect(range.start).toBe(51);
    expect(range.end).toBeGreaterThan(50);

    // Verify page info updated
    const pageInfo = page.locator('.page-info');
    const pageText = await pageInfo.textContent();
    expect(pageText).toMatch(/2 of \d+/);

    // Click previous page — wait for data to load
    const prevButton = page.locator('button[aria-label="Previous page"]');
    rowsLoaded = seeki.pendingRowsResponse();
    await prevButton.click();
    await rowsLoaded;

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
