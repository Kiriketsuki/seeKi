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

  /** Locate the bottom action dock. */
  getActionDock() {
    return this.page.locator('.action-dock[aria-label="Table actions"]');
  }

  /** Locate the dock search panel. */
  getDockSearchPanel() {
    return this.getActionDock().locator('#dock-search-panel');
  }

  /** Locate the dock columns panel. */
  getColumnsPanel() {
    return this.getActionDock().locator('#columns-panel');
  }

  /**
   * Returns a promise that resolves when the next /api/tables/.../rows response arrives.
   * Must be called BEFORE the action that triggers the request.
   */
  pendingRowsResponse(): Promise<import('@playwright/test').Response> {
    return this.page.waitForResponse(
      (resp) =>
        resp.url().includes('/api/tables/') &&
        resp.url().includes('/rows') &&
        resp.ok(),
      { timeout: 10_000 },
    );
  }

  /**
   * Like pendingRowsResponse but accepts any non-500 status (including 4xx).
   * Use in error-state tests where 400 is an expected valid response.
   */
  pendingRowsResponseAny(): Promise<import('@playwright/test').Response> {
    return this.page.waitForResponse(
      (resp) =>
        resp.url().includes('/api/tables/') &&
        resp.url().includes('/rows') &&
        resp.status() < 500,
      { timeout: 10_000 },
    );
  }

  /** Wait for the app to finish loading (spinner gone, grid or wizard visible). */
  async waitForAppReady(): Promise<void> {
    // Wait for either the grid layout or the setup wizard to be visible
    await this.page.waitForFunction(() => {
      const dock = document.querySelector('.action-dock');
      const wizard = document.querySelector('[aria-label="Setup wizard"]');
      return dock !== null || wizard !== null;
    }, { timeout: 15_000 });
  }

  /** Wait for the data grid to have finished loading (works for empty tables too). */
  async waitForGridLoaded(): Promise<void> {
    await this.page.waitForFunction(() => {
      const statusBar = document.querySelector('.statusbar .showing');
      return statusBar !== null && (statusBar.textContent?.trim() ?? '') !== '';
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

  /** Click the dock search button. */
  async clickSearchToggle(): Promise<void> {
    await this.getActionDock().getByRole('button', { name: /search/i }).click();
  }

  /** Click the dock filter button. */
  async clickFilterToggle(): Promise<void> {
    await this.getActionDock().getByRole('button', { name: /filters?/i }).click();
  }

  /** Click the dock columns button. */
  async clickColumnsToggle(): Promise<void> {
    await this.getActionDock().getByRole('button', { name: /columns?/i }).click();
  }

  /** Click the dock export button. */
  async clickExport(): Promise<void> {
    await this.getActionDock().getByRole('button', { name: /export/i }).click();
  }

  /** Get sidebar table names. */
  async getSidebarTableNames(): Promise<string[]> {
    return await this.page.locator('.table-item .table-item-name').allTextContents();
  }

  /** Get the text content of the ActionDock sort announcement live region. */
  getSortAnnouncement() {
    return this.getActionDock().locator('[aria-live]');
  }

  /** Get the four dock action buttons in order: Search, Filters, Columns, Export. */
  getDockButtons() {
    return this.getActionDock().locator('.dock-button');
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
