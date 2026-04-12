/**
 * Shared test fixtures and helpers for SeeKi E2E tests.
 */
import { test as base, expect, type Page } from '@playwright/test';

/** Extended test fixture with SeeKi-specific helpers. */
export const test = base.extend<{ seeki: SeekiHelpers }>({
  seeki: async ({ page }, use) => {
    await use(new SeekiHelpers(page));
  },
});

export { expect };

export class SeekiHelpers {
  constructor(public readonly page: Page) {}

  /** Wait for the app to finish loading (spinner gone, grid or wizard visible). */
  async waitForAppReady(): Promise<void> {
    // Wait for either the grid layout or the setup wizard to be visible
    await this.page.waitForFunction(() => {
      const grid = document.querySelector('.grid-card');
      const wizard = document.querySelector('[aria-label="Setup wizard"]');
      return grid !== null || wizard !== null;
    }, { timeout: 15_000 });
  }

  /** Wait for the data grid to have loaded rows. */
  async waitForGridLoaded(): Promise<void> {
    await this.page.waitForFunction(() => {
      const statusBar = document.querySelector('.statusbar .showing');
      return statusBar && !statusBar.textContent?.includes('Showing 0');
    }, { timeout: 15_000 });
  }

  /** Get the status bar text like "Showing 1 - 50 of 123". */
  async getStatusBarText(): Promise<string> {
    return await this.page.locator('.statusbar .showing').textContent() ?? '';
  }

  /** Parse the total row count from the status bar. */
  async getTotalRows(): Promise<number> {
    const text = await this.getStatusBarText();
    const match = text.match(/of\s+([\d,]+)/);
    return match ? parseInt(match[1].replace(/,/g, ''), 10) : 0;
  }

  /** Parse the current page range from the status bar. */
  async getPageRange(): Promise<{ start: number; end: number }> {
    const text = await this.getStatusBarText();
    const match = text.match(/([\d,]+)\s*-\s*([\d,]+)/);
    return {
      start: match ? parseInt(match[1].replace(/,/g, ''), 10) : 0,
      end: match ? parseInt(match[2].replace(/,/g, ''), 10) : 0,
    };
  }

  /** Click a table in the sidebar to navigate to it. */
  async selectTable(displayName: string): Promise<void> {
    await this.page.locator('.table-item', { hasText: displayName }).click();
    await this.waitForGridLoaded();
  }

  /** Get visible column header labels from the RevoGrid (light DOM). */
  async getVisibleColumnHeaders(): Promise<string[]> {
    const labels = this.page.locator('.sk-grid-header__label');
    await labels.first().waitFor({ state: 'attached', timeout: 5_000 });
    return await labels.allTextContents();
  }

  /** Click the toolbar search button. */
  async clickSearchToggle(): Promise<void> {
    await this.page.locator('.toolbar button[aria-label*="search" i]').first().click();
  }

  /** Click the toolbar filter button. */
  async clickFilterToggle(): Promise<void> {
    await this.page.locator('.toolbar button[aria-label*="filter" i]').first().click();
  }

  /** Click the toolbar columns button. */
  async clickColumnsToggle(): Promise<void> {
    await this.page.locator('.toolbar button[aria-label*="column" i]').first().click();
  }

  /** Click the toolbar export button. */
  async clickExport(): Promise<void> {
    await this.page.locator('.toolbar button[aria-label*="export" i]').first().click();
  }

  /** Get sidebar table names. */
  async getSidebarTableNames(): Promise<string[]> {
    return await this.page.locator('.table-item .table-item-name').allTextContents();
  }

  /** Toggle sidebar collapse/expand. */
  async toggleSidebar(): Promise<void> {
    await this.page.locator('.sidebar .toggle').click();
  }

  /** Check if sidebar is collapsed. */
  async isSidebarCollapsed(): Promise<boolean> {
    const sidebar = this.page.locator('.sidebar');
    return await sidebar.evaluate((el) => el.classList.contains('collapsed'));
  }
}
