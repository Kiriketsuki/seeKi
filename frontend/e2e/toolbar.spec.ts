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
    
    // Verify search bar is visible
    const searchBar = page.locator('div.search-bar');
    await expect(searchBar).toBeVisible();
    
    // Press Escape to close
    await page.keyboard.press('Escape');
    
    // Verify search bar is hidden
    await expect(searchBar).not.toBeVisible();
  });

  test('search toggle via Ctrl+K shortcut', async ({ page }) => {
    const searchBar = page.locator('div.search-bar');
    
    // Press Ctrl+K to open (Cmd+K on macOS)
    const modifier = process.platform === 'darwin' ? 'Meta' : 'Control';
    await page.keyboard.press(`${modifier}+KeyK`);
    
    // Verify search bar appears
    await expect(searchBar).toBeVisible();
    
    // Press Ctrl+K again to close
    await page.keyboard.press(`${modifier}+KeyK`);
    
    // Verify search bar collapses
    await expect(searchBar).not.toBeVisible();
  });

  test('search filters rows', async ({ page, seeki }) => {
    // Open search
    await seeki.clickSearchToggle();

    // Get initial row count from status bar
    const initialTotal = await seeki.getTotalRows();

    // Type a search term
    const searchInput = page.locator('input.search-input');
    await searchInput.fill('1');

    // Wait for debounce (300ms) + API response
    await page.waitForTimeout(500);

    // Verify status bar shows results
    const statusText = await seeki.getStatusBarText();
    expect(statusText).toMatch(/Showing/);

    const filteredTotal = await seeki.getTotalRows();
    expect(filteredTotal).toBeGreaterThanOrEqual(0);
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
    
    // Find the first column checkbox and its label text
    const firstColumnRow = page.locator('label.column-row').first();
    const labelText = await firstColumnRow.locator('span').textContent();
    const checkbox = firstColumnRow.locator('input[type="checkbox"]');
    
    // Verify checkbox is checked
    await expect(checkbox).toBeChecked();
    
    // Uncheck the column
    await checkbox.click();
    
    // Wait for grid to update
    await page.waitForTimeout(200);
    
    // Get visible headers after hiding
    const updatedHeaders = await seeki.getVisibleColumnHeaders();
    
    // Verify the column was removed
    expect(updatedHeaders.length).toBe(initialHeaders.length - 1);
    expect(updatedHeaders).not.toContain(labelText);
  });

  test('column visibility persists across page reload', async ({ page, seeki }) => {
    // Get the current table name from URL or status
    const url = page.url();
    const tableMatch = url.match(/table=([^&]+)/);
    const tableName = tableMatch ? tableMatch[1] : 'unknown';
    
    // Open column dropdown
    await seeki.clickColumnsToggle();
    
    // Get the first column name
    const firstColumnRow = page.locator('label.column-row').first();
    const columnName = await firstColumnRow.locator('span').textContent();
    const checkbox = firstColumnRow.locator('input[type="checkbox"]');
    
    // Hide the column
    await checkbox.click();
    
    // Wait for change to be saved to localStorage
    await page.waitForTimeout(200);
    
    // Verify localStorage was updated
    const storageKey = `sk-column-visibility-${tableName}`;
    const storageValue = await page.evaluate((key) => {
      return localStorage.getItem(key);
    }, storageKey);
    expect(storageValue).toBeTruthy();
    
    // Reload the page
    await page.reload();
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
    
    // Get visible headers after reload
    const headersAfterReload = await seeki.getVisibleColumnHeaders();
    
    // Verify the column is still hidden
    expect(headersAfterReload).not.toContain(columnName);
  });

  test('Show All restores all columns', async ({ page, seeki }) => {
    // Open column dropdown
    await seeki.clickColumnsToggle();
    
    // Hide a column
    const firstCheckbox = page.locator('label.column-row input[type="checkbox"]').first();
    await firstCheckbox.click();
    
    // Wait for update
    await page.waitForTimeout(200);
    
    // Get column count after hiding
    const headersAfterHiding = await seeki.getVisibleColumnHeaders();
    
    // Click "Show All" button
    const showAllButton = page.locator('button.show-all');
    await showAllButton.click();
    
    // Wait for update
    await page.waitForTimeout(200);
    
    // Get column count after show all
    const headersAfterShowAll = await seeki.getVisibleColumnHeaders();
    
    // Verify all columns are restored
    expect(headersAfterShowAll.length).toBeGreaterThan(headersAfterHiding.length);
    
    // Verify all checkboxes are checked
    const allCheckboxes = page.locator('label.column-row input[type="checkbox"]');
    const checkboxCount = await allCheckboxes.count();
    
    for (let i = 0; i < checkboxCount; i++) {
      await expect(allCheckboxes.nth(i)).toBeChecked();
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
