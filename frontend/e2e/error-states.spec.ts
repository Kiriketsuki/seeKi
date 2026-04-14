import { test, expect } from './fixtures';

test.describe('Error States — API Errors', () => {
  test('request for non-existent table returns 404', async ({ page }) => {
    const response = await page.request.get('/api/tables/nonexistent_schema/nonexistent_table_xyz/rows');
    
    expect(response.status()).toBe(404);
    
    const body = await response.json();
    expect(body).toHaveProperty('error');
    expect(body.error).toContain('nonexistent_table_xyz');
    
    // Ensure no internal details are leaked
    const bodyText = JSON.stringify(body).toLowerCase();
    expect(bodyText).not.toContain('stack trace');
    expect(bodyText).not.toContain('panic');
    expect(bodyText).not.toContain('src/');
    expect(bodyText).not.toContain('cargo');
  });

  test('404 response does not leak internal details', async ({ page }) => {
    const response = await page.request.get('/api/tables/nonexistent_schema/nonexistent_table_xyz/columns');
    
    expect(response.status()).toBe(404);
    
    const body = await response.json();
    const bodyText = JSON.stringify(body).toLowerCase();
    
    // Check for database internals that should not be exposed
    expect(bodyText).not.toContain('postgres');
    expect(bodyText).not.toContain('pg_catalog');
    expect(bodyText).not.toContain('stack trace');
    expect(bodyText).not.toContain('panic');
    
    // Should not contain raw SQL
    expect(bodyText).not.toContain('select ');
    expect(bodyText).not.toContain('from ');
    expect(bodyText).not.toContain('where ');
  });
});

test.describe('Error States — SQL Injection Prevention', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('SQL injection in filter input is handled safely', async ({ page, seeki }) => {
    // Get the current table name dynamically from the API
    const tablesResponse = await page.request.get('/api/tables');
    expect(tablesResponse.ok()).toBeTruthy();
    const tablesData = await tablesResponse.json() as { tables: { name: string; schema: string }[] };
    expect(tablesData.tables.length).toBeGreaterThan(0);
    const currentTable = tablesData.tables[0].name;

    // Open filters
    await seeki.clickFilterToggle();

    // Find the first filter input
    const filterInput = page.locator('input[aria-label^="Filter "]').first();
    await expect(filterInput).toBeVisible();

    // Type SQL injection payload — wait for the debounced API response
    const rowsLoaded = seeki.pendingRowsResponseAny();
    await filterInput.fill(`'; DROP TABLE ${currentTable}--`);
    await rowsLoaded;

    // Verify the app is still responsive
    await expect(page.locator('revo-grid')).toBeVisible();

    // Check that no SQL error is shown to the user
    const errorBanner = page.locator('div.error-banner');
    if (await errorBanner.isVisible()) {
      const errorText = await errorBanner.textContent();
      expect(errorText?.toLowerCase()).not.toContain('sql');
      expect(errorText?.toLowerCase()).not.toContain('syntax');
      expect(errorText?.toLowerCase()).not.toContain('drop table');
    }

    // Verify the table still exists and is queryable (injection did NOT execute)
    const verifyResponse = await page.request.get('/api/tables');
    expect(verifyResponse.ok()).toBeTruthy();
    const verifyData = await verifyResponse.json() as { tables: { name: string }[] };
    const tableNames = verifyData.tables.map((t: { name: string }) => t.name);
    expect(tableNames).toContain(currentTable);
  });

  test('SQL injection in search is handled safely', async ({ page, seeki }) => {
    // Capture initial row count before injection attempt
    const initialTotal = await seeki.getTotalRows();

    // Open global search with Ctrl+K
    await page.keyboard.press('Control+k');

    const searchInput = page.locator('input.search-input');
    await expect(searchInput).toBeVisible();

    // Type SQL injection payload — wait for debounced API response
    const rowsLoaded = seeki.pendingRowsResponseAny();
    await searchInput.fill('" OR 1=1; --');
    await rowsLoaded;

    // Verify the app is still responsive
    await expect(page.locator('revo-grid')).toBeVisible();

    // Check that no SQL error is shown
    const errorBanner = page.locator('div.error-banner');
    if (await errorBanner.isVisible()) {
      const errorText = await errorBanner.textContent();
      expect(errorText?.toLowerCase()).not.toContain('sql');
      expect(errorText?.toLowerCase()).not.toContain('syntax');
      expect(errorText?.toLowerCase()).not.toContain('or 1=1');
    }

    // A successful OR 1=1 injection would return MORE rows than the initial count.
    // The search should return <= initial rows (it's a filter, not an expander).
    const searchTotal = await seeki.getTotalRows();
    expect(searchTotal).toBeLessThanOrEqual(initialTotal);
  });

  test('SQL injection in URL params is handled safely', async ({ page }) => {
    // First, get the list of tables to use a valid table name
    const tablesResponse = await page.request.get('/api/tables');
    expect(tablesResponse.ok()).toBeTruthy();
    
    const tablesData = await tablesResponse.json() as { tables: { name: string; schema: string }[] };
    expect(tablesData.tables.length).toBeGreaterThan(0);
    
    const firstTable = tablesData.tables[0];
    
    // Attempt SQL injection via URL params
    const maliciousUrl = `/api/tables/${firstTable.schema}/${firstTable.name}/rows?filter.id=%27%3B+DROP+TABLE+test--`;
    const response = await page.request.get(maliciousUrl);
    
    // Should either succeed with 0 rows or return 400, but never 500
    expect(response.status()).not.toBe(500);
    
    const body = await response.json();
    const bodyText = JSON.stringify(body).toLowerCase();
    
    // Should not contain SQL error messages
    expect(bodyText).not.toContain('drop table');
    expect(bodyText).not.toContain('syntax error');
    expect(bodyText).not.toContain('sql');
    expect(bodyText).not.toContain('pg_');
    expect(bodyText).not.toContain('postgres');
    
    // Should not leak stack traces
    expect(bodyText).not.toContain('stack trace');
    expect(bodyText).not.toContain('panic');
  });
});

test.describe('Error States — Edge Cases', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('very long filter value does not crash', async ({ page, seeki }) => {
    // Open filters
    await seeki.clickFilterToggle();

    // Find the first filter input
    const filterInput = page.locator('input[aria-label^="Filter "]').first();
    await expect(filterInput).toBeVisible();

    // Create a very long string (1000+ chars) — wait for debounced response
    const longString = 'A'.repeat(1500);
    const rowsLoaded = seeki.pendingRowsResponseAny();
    await filterInput.fill(longString);
    await rowsLoaded;
    
    // Verify the app is still responsive
    await expect(page.locator('revo-grid')).toBeVisible();
    
    // The app should handle it gracefully
    const errorBanner = page.locator('div.error-banner');
    if (await errorBanner.isVisible()) {
      const errorText = await errorBanner.textContent();
      // Should be a user-friendly error, not a crash
      expect(errorText).toBeDefined();
      expect(errorText?.toLowerCase()).not.toContain('panic');
      expect(errorText?.toLowerCase()).not.toContain('stack trace');
    }
    
    // Grid should still be functional
    await expect(page.locator('revo-grid')).toBeVisible();
  });

  test('special characters in search do not crash', async ({ page, seeki }) => {
    // Test various special characters and potentially dangerous strings
    const testInputs = [
      'unicode: 你好世界 🚀 ñ ü',
      '😀😁😂🤣😃😄😅😆',
      '<script>alert(1)</script>',
      '"><img src=x onerror=alert(1)>',
      '../../etc/passwd',
      'null byte test',
    ];

    for (const testInput of testInputs) {
      // Open global search with Ctrl+K
      await page.keyboard.press('Control+k');

      const searchInput = page.locator('input.search-input');
      await expect(searchInput).toBeVisible();

      // Clear and type the test input — wait for debounced API response
      await searchInput.clear();
      const rowsLoaded = seeki.pendingRowsResponseAny();
      await searchInput.fill(testInput);
      await rowsLoaded;

      // Verify the app is still responsive
      await expect(page.locator('revo-grid')).toBeVisible();

      // No XSS or dangerous errors
      const errorBanner = page.locator('div.error-banner');
      if (await errorBanner.isVisible()) {
        const errorText = await errorBanner.textContent();
        expect(errorText?.toLowerCase()).not.toContain('script');
        expect(errorText?.toLowerCase()).not.toContain('onerror');
      }

      // Close search for next iteration — wait for panel to disappear
      await page.keyboard.press('Escape');
      await expect(page.locator('#dock-search-panel')).not.toBeVisible();
    }
    
    // Final check: app is still functional
    await expect(page.locator('revo-grid')).toBeVisible();
    const title = await page.title();
    expect(title).toBeDefined();
  });
});
