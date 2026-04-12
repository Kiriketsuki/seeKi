import { test, expect } from './fixtures';

test.describe('Setup Wizard', () => {
  test.describe('Initial State', () => {
    test('wizard is visible in setup mode', async ({ page, seeki }) => {
      await page.goto('/');
      await seeki.waitForAppReady();

      // Verify wizard overlay is present
      const wizardOverlay = page.locator('div.overlay[role="dialog"][aria-modal="true"][aria-label="Setup wizard"]');
      await expect(wizardOverlay).toBeVisible();

      // Verify we're on step 1
      await expect(page.locator('p.step-label')).toHaveText('Step 1 of 4');
      await expect(page.locator('h2.step-title')).toHaveText('Connect your database');

      // Verify progress dots show step 1 as current
      const currentDot = page.locator('div.progress[role="group"] > button.dot.current');
      await expect(currentDot).toHaveCount(1);
    });
  });

  test.describe('Step 1 - Connection', () => {
    test('SSH toggle reveals and hides SSH fields', async ({ page, seeki }) => {
      await page.goto('/');
      await seeki.waitForAppReady();

      // Locate SSH toggle checkbox
      const sshToggle = page.locator('label.toggle-row:has-text("Connect via SSH Tunnel") input[type="checkbox"]');
      
      // SSH fields should be hidden initially
      await expect(page.locator('input#ssh-host')).not.toBeVisible();
      await expect(page.locator('input#ssh-port')).not.toBeVisible();
      await expect(page.locator('input#ssh-user')).not.toBeVisible();

      // Enable SSH tunnel
      await sshToggle.check();

      // SSH fields should now be visible
      await expect(page.locator('input#ssh-host')).toBeVisible();
      await expect(page.locator('input#ssh-port')).toBeVisible();
      await expect(page.locator('input#ssh-user')).toBeVisible();
      await expect(page.locator('select#ssh-auth-method')).toBeVisible();

      // Disable SSH tunnel
      await sshToggle.uncheck();

      // SSH fields should be hidden again
      await expect(page.locator('input#ssh-host')).not.toBeVisible();
      await expect(page.locator('input#ssh-port')).not.toBeVisible();
      await expect(page.locator('input#ssh-user')).not.toBeVisible();
    });

    test('failed connection shows error', async ({ page, seeki }) => {
      await page.goto('/');
      await seeki.waitForAppReady();

      // Mock failed connection
      await page.route('**/api/setup/test-connection', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: false,
            error: 'Connection refused: Invalid hostname'
          })
        });
      });

      // Enter invalid connection string
      await page.locator('input#db-url').fill('postgresql://invalid:5432/db');

      // Click Test Connection
      const testButton = page.locator('button.btn-test[aria-label="Test database connection"]');
      await testButton.click();

      // Wait for error indicator to appear
      const errorIndicator = page.locator('span.test-err');
      await expect(errorIndicator).toBeVisible({ timeout: 5000 });
      await expect(errorIndicator).toContainText('Connection refused');

      // Success indicator should not be visible
      await expect(page.locator('span.test-ok')).not.toBeVisible();

      // Next button should remain disabled
      const nextButton = page.locator('button.btn-next[aria-label="Proceed to table selection"]');
      await expect(nextButton).toBeDisabled();
    });
  });

  test.describe('Step Navigation', () => {
    test('step navigation works (forward and back)', async ({ page, seeki }) => {
      await page.goto('/');
      await seeki.waitForAppReady();

      // Mock successful connection test
      await page.route('**/api/setup/test-connection', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            tables: [
              { name: 'users', estimated_rows: 150, is_system: false },
              { name: 'products', estimated_rows: 300, is_system: false }
            ]
          })
        });
      });

      // ===== Step 1: Connection =====
      await expect(page.locator('p.step-label')).toHaveText('Step 1 of 4');

      // Fill in connection details and test
      await page.locator('input#db-url').fill('postgresql://localhost:5432/testdb');
      await page.locator('button.btn-test[aria-label="Test database connection"]').click();
      
      // Wait for success indicator
      await expect(page.locator('span.test-ok')).toBeVisible({ timeout: 5000 });

      // Next button should now be enabled
      const step1NextButton = page.locator('button.btn-next[aria-label="Proceed to table selection"]');
      await expect(step1NextButton).toBeEnabled();
      await step1NextButton.click();

      // ===== Step 2: Tables =====
      await expect(page.locator('p.step-label')).toHaveText('Step 2 of 4');
      await expect(page.locator('h2.step-title')).toContainText('Tables');

      // Verify table list is visible
      await expect(page.locator('div.table-list[role="listbox"]')).toBeVisible();

      // Select at least one table
      const firstTableCheckbox = page.locator('input[type="checkbox"][aria-label="Include table users"]');
      await firstTableCheckbox.check();

      // Go to step 3
      const step2NextButton = page.locator('button.btn-next[aria-label="Proceed to branding"]');
      await step2NextButton.click();

      // ===== Step 3: Branding =====
      await expect(page.locator('p.step-label')).toHaveText('Step 3 of 4');
      await expect(page.locator('h2.step-title')).toContainText('Branding');

      // Fill in branding details
      await page.locator('input#brand-title').fill('My Test Database');
      await page.locator('input#brand-subtitle').fill('Test Environment');

      // Go to step 4
      const step3NextButton = page.locator('button.btn-next');
      await step3NextButton.click();

      // ===== Step 4: Confirm =====
      await expect(page.locator('p.step-label')).toHaveText('Step 4 of 4');
      await expect(page.locator('h2.step-title')).toContainText('Confirm');

      // ===== Navigate Backwards =====
      
      // Back to step 3
      let backButton = page.locator('button.btn-back').first();
      await backButton.click();
      await expect(page.locator('p.step-label')).toHaveText('Step 3 of 4');

      // Back to step 2
      backButton = page.locator('button.btn-back[aria-label="Go back to connection setup"]');
      await backButton.click();
      await expect(page.locator('p.step-label')).toHaveText('Step 2 of 4');

      // Verify our table selection persisted
      await expect(firstTableCheckbox).toBeChecked();

      // Back to step 1
      backButton = page.locator('button.btn-back').first();
      await backButton.click();
      await expect(page.locator('p.step-label')).toHaveText('Step 1 of 4');

      // Verify connection details persisted
      await expect(page.locator('input#db-url')).toHaveValue('postgresql://localhost:5432/testdb');
      await expect(page.locator('span.test-ok')).toBeVisible();
    });
  });

  test.describe('Complete Setup Flow', () => {
    test('successful connection and save flow', async ({ page, seeki }) => {
      await page.goto('/');
      await seeki.waitForAppReady();

      // Mock API endpoints
      await page.route('**/api/setup/test-connection', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            tables: [
              { name: 'test_table', estimated_rows: 100, is_system: false }
            ]
          })
        });
      });

      await page.route('**/api/setup/save', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ success: true })
        });
      });

      // After save, the app will reload and should be in normal mode
      await page.route('**/api/status', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ mode: 'normal' })
        });
      });

      // ===== Step 1: Connection =====
      await page.locator('input#db-url').fill('postgresql://localhost:5432/testdb');
      await page.locator('button.btn-test[aria-label="Test database connection"]').click();
      await expect(page.locator('span.test-ok')).toBeVisible({ timeout: 5000 });
      await page.locator('button.btn-next[aria-label="Proceed to table selection"]').click();

      // ===== Step 2: Tables =====
      await expect(page.locator('p.step-label')).toHaveText('Step 2 of 4');
      const tableCheckbox = page.locator('input[type="checkbox"][aria-label="Include table test_table"]');
      await tableCheckbox.check();
      await page.locator('button.btn-next[aria-label="Proceed to branding"]').click();

      // ===== Step 3: Branding =====
      await expect(page.locator('p.step-label')).toHaveText('Step 3 of 4');
      await page.locator('input#brand-title').fill('SeeKi Test');
      await page.locator('input#brand-subtitle').fill('Test Database');
      await page.locator('button.btn-next').click();

      // ===== Step 4: Confirm & Save =====
      await expect(page.locator('p.step-label')).toHaveText('Step 4 of 4');

      // Listen for the page reload that happens after save
      const reloadPromise = page.waitForEvent('load');

      // Click Save
      const saveButton = page.locator('button.btn-save');
      await saveButton.click();

      // Wait for reload
      await reloadPromise;

      // After reload, wizard should not be visible (we're in normal mode)
      const wizardOverlay = page.locator('div.overlay[role="dialog"][aria-label="Setup wizard"]');
      await expect(wizardOverlay).not.toBeVisible({ timeout: 5000 });
    });
  });
});
