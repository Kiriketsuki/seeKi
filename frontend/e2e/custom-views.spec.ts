import { test, expect } from './fixtures';

test.describe.serial('Custom views', () => {
  test('create a joined view, save, open, and delete it', async ({ page, seeki }) => {
    // 1. Navigate and wait for app + grid
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    // 2. Click "Create" in the Views section
    await page.locator('.view-list .create-btn').click();

    // 3. Wait for builder
    const builder = page.locator('.builder');
    await expect(builder).toBeVisible();
    await expect(builder.locator('.builder-header h2')).toHaveText('Custom view builder');

    // 4. Set base table to public.vehicle_logs
    const baseTableSelect = builder.locator('label.field').filter({ hasText: 'Base table' }).locator('select');
    await baseTableSelect.selectOption('public.vehicle_logs');

    // 5. Click "Add column" — column picker opens
    await builder.locator('.panel-subheader button.secondary', { hasText: 'Add column' }).click();
    const dialog = page.locator('[role="dialog"][aria-label="Choose a view column"]');
    await expect(dialog).toBeVisible();

    // 6. Source table defaults to public.vehicle_logs. Select column "event_type".
    const sourceTableSelect = dialog.locator('label').filter({ hasText: 'Source table' }).locator('select');
    await expect(sourceTableSelect).toHaveValue('public.vehicle_logs');

    // Wait for columns to load
    const sourceColumnSelect = dialog.locator('label').filter({ hasText: 'Source column' }).locator('select');
    await expect(sourceColumnSelect).not.toBeDisabled();
    await sourceColumnSelect.selectOption('event_type');

    // Click "Add column" submit
    await dialog.locator('.picker-actions button.primary', { hasText: 'Add column' }).click();
    await expect(dialog).not.toBeVisible();

    // 7. Add second column from FK-reachable table: public.vehicles → label
    await builder.locator('.panel-subheader button.secondary', { hasText: 'Add column' }).click();
    await expect(dialog).toBeVisible();

    // Change source table to public.vehicles
    await sourceTableSelect.selectOption('public.vehicles');

    // Wait for columns to load (select becomes enabled with options)
    await expect(sourceColumnSelect).not.toBeDisabled();
    await page.waitForFunction(() => {
      const dlg = document.querySelector('[role="dialog"][aria-label="Choose a view column"]');
      if (!dlg) return false;
      const selects = dlg.querySelectorAll('select');
      // Source column is the second select
      const colSelect = selects[1];
      return colSelect && colSelect.options.length > 0 && !colSelect.disabled;
    });

    await sourceColumnSelect.selectOption('label');
    await dialog.locator('.picker-actions button.primary', { hasText: 'Add column' }).click();
    await expect(dialog).not.toBeVisible();

    // 8. Wait for preview table to appear with rows
    const previewRows = page.locator('table.preview-table tbody tr');
    await previewRows.first().waitFor({ state: 'visible', timeout: 15_000 });
    const previewRowCount = await previewRows.count();
    expect(previewRowCount).toBeGreaterThan(0);

    // 9. Enter view name
    const nameInput = builder.locator('label.field').filter({ hasText: 'Saved view name' }).locator('input');
    await nameInput.fill('E2E Test View');

    // 10. Click "Save view"
    await builder.locator('.builder-actions button.primary', { hasText: 'Save view' }).click();

    // 11. Wait for view to appear in sidebar
    await expect(async () => {
      const viewNames = await seeki.getSidebarViewNames();
      expect(viewNames).toContain('E2E Test View');
    }).toPass({ timeout: 10_000 });

    // 12. App should auto-navigate to the saved view. Wait for grid loaded.
    await seeki.waitForGridLoaded();

    // 13. Verify the view toolbar shows "Read-only saved view" pill
    const viewToolbar = page.locator('.view-toolbar');
    await expect(viewToolbar).toBeVisible();
    await expect(viewToolbar.locator('.view-pill')).toHaveText('Read-only saved view');

    // 14. Verify column headers include expected columns
    const headers = await seeki.getVisibleColumnHeaders();
    expect(headers.length).toBeGreaterThanOrEqual(2);
    // Headers should include the columns we selected (event_type and label)
    const lowerHeaders = headers.map((h) => h.toLowerCase());
    expect(lowerHeaders.some((h) => h.includes('event_type'))).toBe(true);
    expect(lowerHeaders.some((h) => h.includes('label'))).toBe(true);

    // 15. Delete the view via toolbar "Delete view" button
    // The toolbar Delete button calls handleDeleteSavedView directly (no window.confirm)
    await viewToolbar.locator('button.view-action.view-action--danger', { hasText: 'Delete view' }).click();

    // 16. Verify the view disappears from sidebar
    await expect(async () => {
      const viewNames = await seeki.getSidebarViewNames();
      expect(viewNames).not.toContain('E2E Test View');
    }).toPass({ timeout: 10_000 });

    // 17. Verify the app falls back (grid still loaded, no builder visible)
    await seeki.waitForGridLoaded();
    await expect(builder).not.toBeVisible();
  });

  test('create an aggregate view with SUM', async ({ page, seeki }) => {
    // 1. Navigate and wait for app + grid
    await page.goto('/');
    await seeki.waitForAppReady();
    await seeki.waitForGridLoaded();

    // 2. Click "Create" in the Views section
    await page.locator('.view-list .create-btn').click();

    // 3. Wait for builder
    const builder = page.locator('.builder');
    await expect(builder).toBeVisible();

    // 4. Set base table to public.vehicle_logs
    const baseTableSelect = builder.locator('label.field').filter({ hasText: 'Base table' }).locator('select');
    await baseTableSelect.selectOption('public.vehicle_logs');

    const dialog = page.locator('[role="dialog"][aria-label="Choose a view column"]');
    const sourceTableSelect = dialog.locator('label').filter({ hasText: 'Source table' }).locator('select');
    const sourceColumnSelect = dialog.locator('label').filter({ hasText: 'Source column' }).locator('select');
    const aggregateSelect = dialog.locator('label').filter({ hasText: 'Aggregate' }).locator('select');

    // 5. Add column: public.vehicles → label (non-aggregate, used as group-by key)
    await builder.locator('.panel-subheader button.secondary', { hasText: 'Add column' }).click();
    await expect(dialog).toBeVisible();

    await sourceTableSelect.selectOption('public.vehicles');

    // Wait for columns to load
    await expect(sourceColumnSelect).not.toBeDisabled();
    await page.waitForFunction(() => {
      const dlg = document.querySelector('[role="dialog"][aria-label="Choose a view column"]');
      if (!dlg) return false;
      const selects = dlg.querySelectorAll('select');
      const colSelect = selects[1];
      return colSelect && colSelect.options.length > 0 && !colSelect.disabled;
    });

    await sourceColumnSelect.selectOption('label');
    await dialog.locator('.picker-actions button.primary', { hasText: 'Add column' }).click();
    await expect(dialog).not.toBeVisible();

    // 6. Add column: public.vehicle_logs → speed_kmh with SUM aggregate
    await builder.locator('.panel-subheader button.secondary', { hasText: 'Add column' }).click();
    await expect(dialog).toBeVisible();

    // Source table should default to base table (vehicle_logs)
    await expect(sourceTableSelect).toHaveValue('public.vehicle_logs');

    // Wait for columns to load
    await expect(sourceColumnSelect).not.toBeDisabled();
    await page.waitForFunction(() => {
      const dlg = document.querySelector('[role="dialog"][aria-label="Choose a view column"]');
      if (!dlg) return false;
      const selects = dlg.querySelectorAll('select');
      const colSelect = selects[1];
      return colSelect && colSelect.options.length > 0 && !colSelect.disabled;
    });

    await sourceColumnSelect.selectOption('speed_kmh');
    await aggregateSelect.selectOption('SUM');
    await dialog.locator('.picker-actions button.primary', { hasText: 'Add column' }).click();
    await expect(dialog).not.toBeVisible();

    // 7. Wait for preview — should show grouped results (fewer than 200 rows)
    const previewRows = page.locator('table.preview-table tbody tr');
    await previewRows.first().waitFor({ state: 'visible', timeout: 15_000 });

    // 8. Verify preview has rows and they are grouped (fewer than the full 200 vehicle_logs)
    const previewRowCount = await previewRows.count();
    expect(previewRowCount).toBeGreaterThan(0);
    // There are 5 vehicles, so grouped results should have at most 5 rows
    expect(previewRowCount).toBeLessThanOrEqual(5);

    // 9. Enter name, save
    const nameInput = builder.locator('label.field').filter({ hasText: 'Saved view name' }).locator('input');
    await nameInput.fill('E2E Aggregate View');
    await builder.locator('.builder-actions button.primary', { hasText: 'Save view' }).click();

    // 10. Verify it appears in sidebar
    await expect(async () => {
      const viewNames = await seeki.getSidebarViewNames();
      expect(viewNames).toContain('E2E Aggregate View');
    }).toPass({ timeout: 10_000 });

    // 11. Open it and verify grid loads
    await seeki.waitForGridLoaded();
    const viewToolbar = page.locator('.view-toolbar');
    await expect(viewToolbar).toBeVisible();

    // 12. Clean up: delete the view
    await viewToolbar.locator('button.view-action.view-action--danger', { hasText: 'Delete view' }).click();

    await expect(async () => {
      const viewNames = await seeki.getSidebarViewNames();
      expect(viewNames).not.toContain('E2E Aggregate View');
    }).toPass({ timeout: 10_000 });
  });
});
