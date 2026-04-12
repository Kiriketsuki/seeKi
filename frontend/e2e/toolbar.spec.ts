import { test, expect } from './fixtures';

test.describe('Toolbar — Search', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('search toggle via icon click', async ({ page, seeki }) => {
    // Click search button to open
    await seeki.clickSearchToggle();

    // Verify search panel is visible (id="search-panel", conditionally rendered)
    const searchPanel = page.locator('div#search-panel');
    await expect(searchPanel).toBeVisible();

    // Press Escape to close
    await page.keyboard.press('Escape');

    // Verify search panel is hidden (element removed from DOM)
    await expect(searchPanel).not.toBeVisible();
  });

  test('search toggle via Ctrl+K shortcut', async ({ page }) => {
    const searchPanel = page.locator('div#search-panel');

    // Press Ctrl+K to open
    const modifier = process.platform === 'darwin' ? 'Meta' : 'Control';
    await page.keyboard.press(`${modifier}+KeyK`);

    // Verify search panel appears
    await expect(searchPanel).toBeVisible();

    // Press Ctrl+K again to close
    await page.keyboard.press(`${modifier}+KeyK`);

    // Verify search panel collapses
    await expect(searchPanel).not.toBeVisible();
  });

  test('search filters rows', async ({ page, seeki }) => {
    // Open search
    await seeki.clickSearchToggle();

    // Get initial row count from status bar
    const initialTotal = await seeki.getTotalRows();

    // Type a search term — wait for debounced API response
    const searchInput = page.locator('input.search-input');
    const rowsLoaded = seeki.pendingRowsResponse();
    await searchInput.fill('1');
    await rowsLoaded;

    // Verify status bar shows results
    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(/Showing/);

    const filteredTotal = await seeki.getTotalRows();
    expect(filteredTotal).toBeLessThanOrEqual(initialTotal);
  });
});

test.describe('Toolbar — Column Visibility', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('column dropdown opens and closes', async ({ page, seeki }) => {
    const columnsPanel = page.locator('div#columns-panel.dropdown[role="region"]');

    // Initially hidden
    await expect(columnsPanel).not.toBeVisible();

    // Click columns button to open
    await seeki.clickColumnsToggle();

    // Verify panel is visible
    await expect(columnsPanel).toBeVisible();

    // Click columns button again to close
    await seeki.clickColumnsToggle();

    // Verify panel is hidden
    await expect(columnsPanel).not.toBeVisible();

    // Open again
    await seeki.clickColumnsToggle();
    await expect(columnsPanel).toBeVisible();

    // Press Escape to close
    await page.keyboard.press('Escape');

    // Verify panel is hidden
    await expect(columnsPanel).not.toBeVisible();
  });

  test('hiding a column removes it from grid', async ({ page, seeki }) => {
    // Open column dropdown
    await seeki.clickColumnsToggle();

    // Get all visible column headers before hiding
    const initialHeaders = await seeki.getVisibleColumnHeaders();
    expect(initialHeaders.length).toBeGreaterThan(0);

    // The column rows are buttons (not labels with checkboxes)
    const firstColumnRow = page.locator('button.column-row').first();
    const labelText = await firstColumnRow.locator('.column-label').textContent();

    // Verify column is visible (aria-pressed="true")
    await expect(firstColumnRow).toHaveAttribute('aria-pressed', 'true');

    // Click to hide the column — wait for header count to change
    await firstColumnRow.click();
    await page.waitForFunction(
      (expected) => document.querySelectorAll('.sk-grid-header__label').length === expected,
      initialHeaders.length - 1,
    );

    // Get visible headers after hiding
    const updatedHeaders = await seeki.getVisibleColumnHeaders();

    // Verify the column was removed
    expect(updatedHeaders.length).toBe(initialHeaders.length - 1);
    expect(updatedHeaders).not.toContain(labelText?.trim());
  });

  test('column visibility persists across page reload', async ({ page, seeki }) => {
    // Open column dropdown
    await seeki.clickColumnsToggle();

    // Get the first column button and its label
    const firstColumnRow = page.locator('button.column-row').first();
    const columnName = await firstColumnRow.locator('.column-label').textContent();

    // Hide the column — wait for localStorage to be written
    await firstColumnRow.click();
    await page.waitForFunction(() => {
      return Object.keys(localStorage).some(k => k.startsWith('sk-column-visibility-'));
    });

    // Verify a column visibility key was written to localStorage
    const storageValue = await page.evaluate(() => {
      const key = Object.keys(localStorage).find(k => k.startsWith('sk-column-visibility-'));
      return key ? localStorage.getItem(key) : null;
    });
    expect(storageValue).toBeTruthy();

    // Reload the page
    await page.reload();
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    // Get visible headers after reload
    const headersAfterReload = await seeki.getVisibleColumnHeaders();

    // Verify the column is still hidden
    expect(headersAfterReload).not.toContain(columnName?.trim());
  });

  test('Show All restores all columns', async ({ page, seeki }) => {
    // Open column dropdown
    await seeki.clickColumnsToggle();

    // Get initial column count so we can wait for changes
    const initialHeaders = await seeki.getVisibleColumnHeaders();

    // Hide the first column — wait for header count to decrease
    const firstColumnRow = page.locator('button.column-row').first();
    await firstColumnRow.click();
    await page.waitForFunction(
      (expected) => document.querySelectorAll('.sk-grid-header__label').length === expected,
      initialHeaders.length - 1,
    );

    // Get column count after hiding
    const headersAfterHiding = await seeki.getVisibleColumnHeaders();

    // Click "Show All" button — wait for all columns to restore
    const showAllButton = page.locator('button.show-all');
    await showAllButton.click();
    await page.waitForFunction(
      (expected) => document.querySelectorAll('.sk-grid-header__label').length === expected,
      initialHeaders.length,
    );

    // Get column count after show all
    const headersAfterShowAll = await seeki.getVisibleColumnHeaders();

    // Verify all columns are restored
    expect(headersAfterShowAll.length).toBeGreaterThan(headersAfterHiding.length);

    // Verify all column rows have aria-pressed="true"
    const allColumnRows = page.locator('button.column-row');
    const rowCount = await allColumnRows.count();

    for (let i = 0; i < rowCount; i++) {
      await expect(allColumnRows.nth(i)).toHaveAttribute('aria-pressed', 'true');
    }
  });
});

test.describe('Toolbar — CSV Export', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('CSV export triggers file download', async ({ page, seeki }) => {
    // Set up download listener
    const downloadPromise = page.waitForEvent('download');

    // Click export button
    await seeki.clickExport();

    // Wait for download to start
    const download = await downloadPromise;

    // Verify filename has .csv extension
    const filename = download.suggestedFilename();
    expect(filename).toMatch(/\.csv$/i);

    // Optionally verify the download completes
    const path = await download.path();
    expect(path).toBeTruthy();
  });
});
