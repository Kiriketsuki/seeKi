import { test, expect } from './fixtures';

test.describe('Navigation — Default Load', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
  });

  test('sidebar shows the Data workspace label', async ({ page }) => {
    await expect(page.locator('[data-testid="sidebar-mode-data"]').first()).toContainText('Data');
  });

  test('default table loads on app start', async ({ page, seeki }) => {
    // Wait for the grid to be loaded
    await seeki.waitForGridLoaded();

    // Verify a table is selected in the sidebar (has .active class)
    const activeTable = page.locator('button.table-item.active');
    await expect(activeTable).toBeVisible();

    // Verify the grid shows data (status bar should show row count)
    const statusBar = await seeki.getStatusBarText();
    expect(statusBar).toMatch(/(?:Showing \d[\d,]* - \d[\d,]* of|Loaded \d[\d,]* of) \d[\d,]*/);
  });
});

test.describe('Navigation — Table Switching', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();
  });

  test('clicking a different table loads its data', async ({ page, seeki }) => {
    // Get the list of sidebar tables
    const tableNames = await seeki.getSidebarTableNames();
    
    // Skip if there aren't at least 2 tables
    test.skip(tableNames.length < 2, 'Test requires at least 2 tables in the database');

    // Get the second table name
    const secondTableName = tableNames[1];

    // Click the second table
    await seeki.selectTable(secondTableName);
    await seeki.waitForGridLoaded();

    // Verify the clicked table now has .active class
    const activeTable = page.locator('button.table-item.active');
    await expect(activeTable).toContainText(secondTableName);

    // Verify the status bar updates (should show data for the new table)
    const statusBar = await seeki.getStatusBarText();
    expect(statusBar).toMatch(/(?:Showing \d[\d,]* - \d[\d,]* of|Loaded \d[\d,]* of) \d[\d,]*/);
  });

  test('switching tables resets search state and toolbar defaults', async ({ page, seeki }) => {
    const tableNames = await seeki.getSidebarTableNames();
    test.skip(tableNames.length < 2, 'Test requires at least 2 tables in the database');

    const firstHeader = page.locator('[role="columnheader"]').first();
    const firstHeaderState = firstHeader.locator('.sk-grid-header');
    const sortGlyph = firstHeader.locator('.sk-grid-header__sort');
    let rowsLoaded = seeki.pendingRowsResponse();
    await firstHeader.click();
    let rowsResponse = await rowsLoaded;

    // Verify the sort request included sort params
    expect(rowsResponse.request().url()).toContain('sort_direction=asc');
    await expect(page.locator('.action-dock [aria-live]')).toHaveText(/ascending$/);
    await expect(sortGlyph).toHaveText('↑');
    await expect(firstHeaderState).toHaveAttribute('aria-sort', 'ascending');

    const secondTableName = tableNames[1];
    rowsLoaded = seeki.pendingRowsResponse();
    await seeki.selectTable(secondTableName);
    rowsResponse = await rowsLoaded;

    // Verify sort is reset on table switch — the new rows request should be clean
    expect(rowsResponse.request().url()).not.toContain('sort_direction=');
    expect(rowsResponse.request().url()).not.toContain('sort_column=');
    // sort cleared on table switch — live region should be empty
    await expect(page.locator('.action-dock [aria-live]')).toHaveText('');
    await expect(page.locator('.sk-grid-header__sort')).toHaveCount(0);
    await expect(page.locator('[role="columnheader"]').first().locator('.sk-grid-header')).not.toHaveAttribute('aria-sort');
  });

  test('switching to settings and back preserves the current table workspace', async ({ page, seeki }) => {
    const tableNames = await seeki.getSidebarTableNames();
    test.skip(tableNames.length < 2, 'Test requires at least 2 tables in the database');

    const secondTableName = tableNames[1];
    await seeki.selectTable(secondTableName);

    await seeki.clickSearchToggle();
    const searchInput = page.locator('.search-input');
    await expect(searchInput).toBeVisible();
    const rowsLoaded = seeki.pendingRowsResponse();
    await searchInput.fill('a');
    await rowsLoaded;
    await expect(searchInput).toHaveValue('a');

    await seeki.openSettings();
    await expect(page.locator('h1')).toContainText('Updates');

    await seeki.openTables();
    await seeki.waitForGridLoaded();

    await expect(page.locator('button.table-item.active')).toContainText(secondTableName);
    await expect(page.locator('.search-input')).toHaveValue('a');
  });
});

test.describe('Navigation — Sidebar Search', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
  });

  test('sidebar search filters the table list', async ({ page, seeki }) => {
    const tableNames = await seeki.getSidebarTableNames();
    test.skip(tableNames.length === 0, 'Test requires at least 1 table in the database');

    // Get the first table name and use part of it for search
    const firstTableName = tableNames[0];
    const searchTerm = firstTableName.slice(0, 3); // First 3 characters

    // Type in the search input
    const searchInput = page.locator('input.table-search-input');
    await searchInput.fill(searchTerm);

    // Wait for sidebar filter to narrow the list
    await page.waitForFunction(
      ({ count, term }) => {
        const items = document.querySelectorAll('.table-item .table-item-name');
        // Either the count decreased or all visible items contain the term
        return items.length <= count &&
          Array.from(items).every(el => el.textContent?.toLowerCase().includes(term));
      },
      { count: tableNames.length, term: searchTerm.toLowerCase() },
    );

    // Verify only matching tables remain visible
    const visibleTables = await seeki.getSidebarTableNames();
    
    // All visible tables should contain the search term (case-insensitive)
    for (const tableName of visibleTables) {
      expect(tableName.toLowerCase()).toContain(searchTerm.toLowerCase());
    }

    // Verify we filtered something out (unless all tables match)
    if (tableNames.length > 1) {
      expect(visibleTables.length).toBeLessThanOrEqual(tableNames.length);
    }
  });

  test('sidebar search with no results shows empty state', async ({ page }) => {
    const searchInput = page.locator('input.table-search-input');
    const nonsenseString = 'xyzabc123nonexistent';
    
    await searchInput.fill(nonsenseString);

    // Verify empty state appears (auto-retries until visible)
    const emptyState = page.locator('div.empty-state');
    await expect(emptyState).toBeVisible();
    await expect(emptyState).toContainText(nonsenseString);
  });
});

test.describe('Navigation — Sidebar Collapse', () => {
  test.beforeEach(async ({ page, seeki }) => {
    await page.goto('/');
    await seeki.waitForAppReady();
  });

  test('sidebar collapse and expand', async ({ page, seeki }) => {
    // Verify sidebar is initially expanded
    const sidebar = page.locator('aside.sidebar');
    await expect(sidebar).not.toHaveClass(/collapsed/);

    // Get initial toggle button label
    const toggleButton = page.locator('button.toggle');
    await expect(toggleButton).toHaveAttribute('aria-label', 'Collapse sidebar');

    // Click to collapse — expect() auto-retries until class appears
    await seeki.toggleSidebar();
    await expect(sidebar).toHaveClass(/collapsed/);
    await expect(toggleButton).toHaveAttribute('aria-label', 'Expand sidebar');

    // Click to expand — expect() auto-retries until class is removed
    await seeki.toggleSidebar();
    await expect(sidebar).not.toHaveClass(/collapsed/);
    await expect(toggleButton).toHaveAttribute('aria-label', 'Collapse sidebar');
  });

  test('sidebar collapse state persists across reload', async ({ page, seeki }) => {
    // Collapse the sidebar — wait for localStorage to be written
    await seeki.toggleSidebar();
    await page.waitForFunction(() => localStorage.getItem('sk-sidebar-collapsed') === 'true');

    // Verify it's collapsed
    const isCollapsed = await seeki.isSidebarCollapsed();
    expect(isCollapsed).toBe(true);

    // Verify localStorage key is set
    const storageValue = await page.evaluate(() => {
      return localStorage.getItem('sk-sidebar-collapsed');
    });
    expect(storageValue).toBe('true');

    // Reload the page
    await page.reload();
    await seeki.waitForAppReady();

    // Verify sidebar remains collapsed
    const isStillCollapsed = await seeki.isSidebarCollapsed();
    expect(isStillCollapsed).toBe(true);

    const sidebar = page.locator('aside.sidebar');
    await expect(sidebar).toHaveClass(/collapsed/);
  });
});
