/** localStorage key for sidebar collapsed state — used by App.svelte (read) and Sidebar.svelte (write). */
export const SIDEBAR_COLLAPSED_KEY = 'sk-sidebar-collapsed';

/** localStorage key prefix for per-table column visibility state. */
export const COLUMN_VISIBILITY_KEY_PREFIX = 'sk-column-visibility-';

/** localStorage key for the persistent Data workspace panel layout. */
export const DATA_PANELS_LAYOUT_KEY = 'sk-data-panels-layout';

/** localStorage key prefix for per-surface auto-refresh preferences. */
export const GRID_REFRESH_KEY_PREFIX = 'sk-grid-refresh-';

/** Supported auto-refresh intervals for table and saved-view surfaces. */
export const GRID_REFRESH_INTERVALS = [0, 15_000, 60_000, 300_000] as const;
