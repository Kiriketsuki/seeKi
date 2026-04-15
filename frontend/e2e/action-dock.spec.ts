import { test, expect } from './fixtures';

test.describe('Action Dock — Shell', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('dock is visible in normal table view and toolbar is absent', async ({ page, seeki }) => {
    const dock = seeki.getActionDock();

    await expect(dock).toBeVisible();
    await expect(page.locator('.toolbar')).toHaveCount(0);
    await expect(page.locator('.statusbar')).toBeVisible();
  });
});

test.describe('Action Dock — Filters', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('filters toggle shows and hides the filter row', async ({ page, seeki }) => {
    const dock = seeki.getActionDock();
    const filterButton = dock.getByRole('button', { name: /filters?/i });
    const filterInputs = page.locator('[role="columnheader"] input[aria-label^="Filter"]');

    await expect(filterButton).toHaveAttribute('aria-expanded', 'false');
    await expect(filterInputs.first()).not.toBeVisible();

    await seeki.clickFilterToggle();
    await expect(filterButton).toHaveAttribute('aria-expanded', 'true');
    await expect(filterInputs.first()).toBeVisible();

    await seeki.clickFilterToggle();
    await expect(filterButton).toHaveAttribute('aria-expanded', 'false');
    await expect(filterInputs.first()).not.toBeVisible();
  });

  test('Escape closes filters and restores focus to filter button', async ({ page, seeki }) => {
    const dock = seeki.getActionDock();
    const filterButton = dock.getByRole('button', { name: /filters?/i });
    const filterInputs = page.locator('[role="columnheader"] input[aria-label^="Filter"]');

    await seeki.clickFilterToggle();
    await expect(filterButton).toHaveAttribute('aria-expanded', 'true');
    await expect(filterInputs.first()).toBeVisible();

    await page.keyboard.press('Escape');

    await expect(filterButton).toHaveAttribute('aria-expanded', 'false');
    await expect(filterInputs.first()).not.toBeVisible();
    await expect(filterButton).toBeFocused();
  });
});

test.describe('Action Dock — Search', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('search toggle via icon click', async ({ page, seeki }) => {
    const dock = seeki.getActionDock();
    const searchButton = dock.getByRole('button', { name: /search/i });
    const searchPanel = seeki.getDockSearchPanel();

    await seeki.clickSearchToggle();
    await expect(searchPanel).toBeVisible();
    await expect(searchPanel.locator('input.search-input')).toBeFocused();

    await page.keyboard.press('Escape');
    await expect(searchPanel).not.toBeVisible();

    await expect(searchButton).toBeFocused();
  });

  test('search toggle via Ctrl+K shortcut', async ({ page, seeki }) => {
    const searchPanel = seeki.getDockSearchPanel();

    const modifier = process.platform === 'darwin' ? 'Meta' : 'Control';
    await page.keyboard.press(`${modifier}+KeyK`);

    await expect(searchPanel).toBeVisible();

    await page.keyboard.press(`${modifier}+KeyK`);

    await expect(searchPanel).not.toBeVisible();
  });

  test('search filters rows', async ({ page, seeki }) => {
    await seeki.clickSearchToggle();

    const initialTotal = await seeki.getTotalRows();

    const searchInput = seeki.getDockSearchPanel().locator('input.search-input');
    const rowsLoaded = seeki.pendingRowsResponse();
    await searchInput.fill('1');
    await rowsLoaded;

    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(/Showing/);

    const filteredTotal = await seeki.getTotalRows();
    expect(filteredTotal).toBeLessThanOrEqual(initialTotal);
  });
});

test.describe('Action Dock — Column Visibility', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('column dropdown opens and closes', async ({ page, seeki }) => {
    const dock = seeki.getActionDock();
    const columnsButton = dock.getByRole('button', { name: /columns?/i });
    const columnsPanel = seeki.getColumnsPanel();

    await expect(columnsPanel).not.toBeVisible();

    await seeki.clickColumnsToggle();
    await expect(columnsPanel).toBeVisible();

    await seeki.clickColumnsToggle();
    await expect(columnsPanel).not.toBeVisible();

    await seeki.clickColumnsToggle();
    await expect(columnsPanel).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(columnsPanel).not.toBeVisible();
    await expect(columnsButton).toBeFocused();
  });

  test('hiding a column removes it from grid', async ({ page, seeki }) => {
    await seeki.clickColumnsToggle();

    const initialHeaders = await seeki.getVisibleColumnHeaders();
    expect(initialHeaders.length).toBeGreaterThan(0);

    const firstColumnRow = page.locator('button.column-row').first();
    await expect(firstColumnRow).toHaveAttribute('aria-pressed', 'true');

    await firstColumnRow.click();
    await page.waitForFunction(
      (expected) => document.querySelectorAll('.sk-grid-header__label').length === expected,
      initialHeaders.length - 1,
    );
    await expect(firstColumnRow).toHaveAttribute('aria-pressed', 'false');

    const updatedHeaders = await seeki.getVisibleColumnHeaders();

    expect(updatedHeaders.length).toBe(initialHeaders.length - 1);
  });

  test('column visibility persists across page reload', async ({ page, seeki }) => {
    await seeki.clickColumnsToggle();

    const firstColumnRow = page.locator('button.column-row').first();
    const columnName = await firstColumnRow.locator('.column-label').textContent();

    await firstColumnRow.click();
    await page.waitForFunction(() => {
      return Object.keys(localStorage).some(k => k.startsWith('sk-column-visibility-'));
    });

    const storageValue = await page.evaluate(() => {
      const key = Object.keys(localStorage).find(k => k.startsWith('sk-column-visibility-'));
      return key ? localStorage.getItem(key) : null;
    });
    expect(storageValue).toBeTruthy();

    await page.reload();
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    const headersAfterReload = await seeki.getVisibleColumnHeaders();

    expect(headersAfterReload).not.toContain(columnName?.trim());
  });

  test('Show All restores all columns', async ({ page, seeki }) => {
    await seeki.clickColumnsToggle();

    const initialHeaders = await seeki.getVisibleColumnHeaders();

    const firstColumnRow = page.locator('button.column-row').first();
    await firstColumnRow.click();
    await page.waitForFunction(
      (expected) => document.querySelectorAll('.sk-grid-header__label').length === expected,
      initialHeaders.length - 1,
    );

    const headersAfterHiding = await seeki.getVisibleColumnHeaders();

    const showAllButton = page.locator('button.show-all');
    await showAllButton.click();
    await page.waitForFunction(
      (expected) => document.querySelectorAll('.sk-grid-header__label').length === expected,
      initialHeaders.length,
    );

    const headersAfterShowAll = await seeki.getVisibleColumnHeaders();

    expect(headersAfterShowAll.length).toBeGreaterThan(headersAfterHiding.length);

    const allColumnRows = page.locator('button.column-row');
    const rowCount = await allColumnRows.count();

    for (let i = 0; i < rowCount; i++) {
      await expect(allColumnRows.nth(i)).toHaveAttribute('aria-pressed', 'true');
    }
  });
});

test.describe('Action Dock — Keyboard Navigation', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('ArrowRight cycles through dock buttons', async ({ page, seeki }) => {
    const buttons = seeki.getDockButtons();
    const search = buttons.nth(0);
    const filters = buttons.nth(1);
    const columns = buttons.nth(2);
    const exportBtn = buttons.nth(3);

    // Start focus on search (index 0)
    await search.focus();
    await expect(search).toBeFocused();

    await page.keyboard.press('ArrowRight');
    await expect(filters).toBeFocused();

    await page.keyboard.press('ArrowRight');
    await expect(columns).toBeFocused();

    await page.keyboard.press('ArrowRight');
    await expect(exportBtn).toBeFocused();

    // Wraps back to search
    await page.keyboard.press('ArrowRight');
    await expect(search).toBeFocused();
  });

  test('ArrowLeft wraps from first to last button', async ({ page, seeki }) => {
    const buttons = seeki.getDockButtons();
    const search = buttons.nth(0);
    const exportBtn = buttons.nth(3);

    await search.focus();
    await page.keyboard.press('ArrowLeft');
    await expect(exportBtn).toBeFocused();
  });

  test('Home and End jump to first and last buttons', async ({ page, seeki }) => {
    const buttons = seeki.getDockButtons();
    const search = buttons.nth(0);
    const columns = buttons.nth(2);
    const exportBtn = buttons.nth(3);

    await columns.focus();

    await page.keyboard.press('End');
    await expect(exportBtn).toBeFocused();

    await page.keyboard.press('Home');
    await expect(search).toBeFocused();
  });

  test('Tab moves focus out of the toolbar', async ({ page, seeki }) => {
    const buttons = seeki.getDockButtons();
    const search = buttons.nth(0);

    await search.focus();
    await page.keyboard.press('Tab');

    // Focus should have left the dock toolbar entirely
    await expect(search).not.toBeFocused();
    await expect(buttons.nth(1)).not.toBeFocused();
    await expect(buttons.nth(2)).not.toBeFocused();
    await expect(buttons.nth(3)).not.toBeFocused();
  });
});

test.describe('Action Dock — CSV Export', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('CSV export triggers file download', async ({ page, seeki }) => {
    const searchPanel = seeki.getDockSearchPanel();
    const columnsPanel = seeki.getColumnsPanel();

    const downloadPromise = page.waitForEvent('download');

    await seeki.clickExport();

    const download = await downloadPromise;
    const filename = download.suggestedFilename();
    expect(filename).toMatch(/\.csv$/i);

    const path = await download.path();
    expect(path).toBeTruthy();

    await expect(searchPanel).not.toBeVisible();
    await expect(columnsPanel).not.toBeVisible();
  });
});
