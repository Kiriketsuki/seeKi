<script lang="ts">
  import { type Snippet } from 'svelte';
  import { ChevronLeft, LayoutGrid, Settings } from 'lucide-svelte';
  import { SIDEBAR_COLLAPSED_KEY } from '../lib/constants';
  import type { SidebarMode } from '../lib/types';

  let {
    collapsed = $bindable(false),
    onToggle,
    onSelectMode,
    title = 'SeeKi',
    subtitle = '',
    updateAvailable = false,
    onSettingsClick = () => {},
    mode = 'tables',
    showModeSwitch = false,
    showSettingsBadge = false,
    children,
  }: {
    collapsed: boolean;
    onToggle: () => void;
    onSelectMode?: (mode: SidebarMode) => void;
    title: string;
    subtitle: string;
    updateAvailable?: boolean;
    onSettingsClick?: () => void;
    mode?: SidebarMode;
    showModeSwitch?: boolean;
    showSettingsBadge?: boolean;
    children?: Snippet;
  } = $props();

  function handleToggle() {
    const nextCollapsed = !collapsed;
    onToggle();
    localStorage.setItem(SIDEBAR_COLLAPSED_KEY, String(nextCollapsed));
  }

  function selectMode(nextMode: SidebarMode) {
    onSelectMode?.(nextMode);
  }
</script>

<aside class="sidebar" class:collapsed data-testid="app-sidebar">
  <div class="header">
    {#if collapsed}
      <!-- collapsed: logo mark IS the expand button — no separate chevron -->
      <button
        class="mark-toggle"
        onclick={handleToggle}
        aria-label="Expand sidebar"
        data-testid="sidebar-toggle"
      >
        <img class="mark mark-collapsed" src="/logo-mark.svg" alt="SeeKi" width="22" height="22" />
      </button>
    {:else}
      <div class="branding">
        <div class="title-row">
          <img class="mark" src="/logo-mark.svg" alt="" aria-hidden="true" width="22" height="22" />
          <span class="title">{title}</span>
        </div>
        {#if subtitle}
          <span class="subtitle">{subtitle}</span>
        {/if}
      </div>
      <!-- expanded: chevron to collapse -->
      <button
        class="toggle"
        onclick={handleToggle}
        aria-label="Collapse sidebar"
        data-testid="sidebar-toggle"
      >
        <ChevronLeft size={16} />
      </button>
    {/if}
  </div>

  {#if showModeSwitch}
    {#if collapsed}
      <div class="collapsed-modes" class:settings-active={mode === 'settings'} aria-label="Workspace mode">
        <button
          type="button"
          class="mode-icon"
          class:active={mode === 'tables'}
          aria-label="Show data workspace"
          aria-selected={mode === 'tables'}
          role="tab"
          onclick={() => selectMode('tables')}
          data-testid="sidebar-mode-data"
        >
          <LayoutGrid size={16} />
        </button>
        <button
          type="button"
          class="mode-icon"
          class:active={mode === 'settings'}
          aria-label="Show settings workspace"
          aria-selected={mode === 'settings'}
          role="tab"
          onclick={() => selectMode('settings')}
          data-testid="sidebar-mode-settings"
        >
          <span class="badge-wrapper">
            <Settings size={16} />
            {#if showSettingsBadge}
              <span class="mode-badge"></span>
            {/if}
          </span>
        </button>
      </div>
    {:else}
      <div class="mode-switch" class:settings-active={mode === 'settings'} role="tablist" aria-label="Workspace mode">
        <button
          type="button"
          class="mode-button"
          class:active={mode === 'tables'}
          role="tab"
          aria-selected={mode === 'tables'}
          onclick={() => selectMode('tables')}
          data-testid="sidebar-mode-data"
        >
          Data
        </button>
        <button
          type="button"
          class="mode-button"
          class:active={mode === 'settings'}
          role="tab"
          aria-selected={mode === 'settings'}
          onclick={() => selectMode('settings')}
          data-testid="sidebar-mode-settings"
        >
          <span class="badge-wrapper">
            Settings
            {#if showSettingsBadge}
              <span class="mode-badge"></span>
            {/if}
          </span>
        </button>
      </div>
    {/if}
  {/if}

  <div class="content">
    {#if children}
      {@render children()}
    {/if}
  </div>

  {#if !collapsed}
    <div class="footer">
      <span class="footer-text">Powered by SeeKi</span>
    </div>
  {/if}
</aside>

<style>
  /* ── Sidebar redesign: frosted-quartz slab (width from tokens.css --sk-sidebar-width) ── */
  .sidebar {
    width: var(--sk-sidebar-width);
    min-width: var(--sk-sidebar-width);
    height: 100vh;
    position: relative;
    display: flex;
    flex-direction: column;
    /* frosted quartz: gradient overlay on glass base */
    background:
      linear-gradient(180deg, rgba(var(--marble-frost-rgb), 0.26) 0%, rgba(var(--marble-frost-rgb), 0.10) 100%),
      var(--sk-glass-sidebar);
    backdrop-filter: var(--sk-glass-sidebar-blur);
    -webkit-backdrop-filter: var(--sk-glass-sidebar-blur);
    border-right: 1px solid var(--sk-border);
    box-shadow: 1px 0 0 rgba(255, 255, 255, 0.45), 8px 0 28px rgba(var(--marble-vein-rgb), 0.045);
    transition: width 0.2s ease, min-width 0.2s ease;
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: var(--sk-sidebar-collapsed);
    min-width: var(--sk-sidebar-collapsed);
  }

  /* ── Header ── */
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-space-lg);
    border-bottom: 1px solid var(--sk-border-light);
  }

  .collapsed .header {
    justify-content: center;
    padding: var(--sk-space-md);
  }

  .branding {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    min-width: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    min-width: 0;
  }

  .mark {
    flex-shrink: 0;
    display: block;
  }

  /* collapsed: logo mark IS the expand button (hover affordance) */
  .mark-collapsed {
    display: block;
  }

  .title {
    font-size: var(--sk-font-size-lg);
    font-weight: 600;
    color: var(--sk-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .subtitle {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .toggle,
  .mode-icon,
  .mode-button {
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    cursor: pointer;
  }

  /* expanded: chevron to collapse */
  .toggle {
    width: 24px;
    height: 24px;
    background: none;
    color: var(--sk-muted);
    border-radius: var(--sk-radius-sm);
    flex-shrink: 0;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .toggle:hover {
    background: var(--sk-border);
    color: var(--sk-text);
  }

  .toggle:focus-visible {
    outline: 2px solid var(--sk-data-strong);
    outline-offset: 2px;
  }

  /* collapsed: mark-toggle wraps the logo and acts as the expand button */
  .mark-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    padding: 0;
    background: none;
    border: none;
    border-radius: var(--sk-radius-md);
    cursor: pointer;
    transition: background 0.12s ease, box-shadow 0.12s ease;
  }

  .mark-toggle:hover {
    background: var(--sk-border);
    box-shadow: inset 0 0 0 1px var(--sk-border-light);
  }

  .mark-toggle:focus-visible {
    outline: 2px solid var(--sk-data-strong);
    outline-offset: 2px;
  }

  /* ── Mode switch (expanded) — amber sliding pill ── */
  .mode-switch {
    position: relative;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xs);
    background: rgba(var(--marble-vein-rgb), 0.05);
    border-radius: var(--sk-radius-lg);
    margin: var(--sk-space-sm) var(--sk-space-sm) 0;
  }

  .mode-switch::before {
    content: '';
    position: absolute;
    top: var(--sk-space-xs);
    left: var(--sk-space-xs);
    width: calc(50% - var(--sk-space-xs) * 1.5);
    height: calc(100% - var(--sk-space-xs) * 2);
    background: rgba(var(--marble-count-rgb), 0.18);
    border-radius: var(--sk-radius-md);
    box-shadow: 0 1px 3px rgba(var(--marble-count-rgb), 0.15);
    transition: transform 200ms cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 0;
  }

  .mode-switch.settings-active::before {
    transform: translateX(calc(100% + var(--sk-space-xs)));
  }

  .mode-button {
    position: relative;
    z-index: 1;
    border-radius: var(--sk-radius-md);
    background: transparent;
    color: var(--sk-secondary-strong);
    padding: var(--sk-space-sm);
    font: inherit;
    font-weight: 500;
    transition: color 150ms ease;
  }

  .mode-button.active {
    color: var(--sk-text);
  }

  .mode-button:focus-visible {
    outline: 2px solid var(--sk-data-strong);
    outline-offset: 2px;
  }

  /* ── Mode switch (collapsed) ── */
  .collapsed-modes {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-sm) 0;
    border-bottom: 1px solid var(--sk-border-light);
  }

  .collapsed-modes::before {
    content: '';
    position: absolute;
    top: var(--sk-space-sm);
    left: 50%;
    transform: translateX(-50%) translateY(0);
    width: 32px;
    height: 32px;
    background: rgba(var(--marble-count-rgb), 0.18);
    border-radius: var(--sk-radius-md);
    box-shadow: 0 1px 3px rgba(var(--marble-count-rgb), 0.15);
    transition: transform 200ms cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 0;
  }

  .collapsed-modes.settings-active::before {
    transform: translateX(-50%) translateY(calc(32px + var(--sk-space-xs)));
  }

  .mode-icon {
    position: relative;
    z-index: 1;
    width: 32px;
    height: 32px;
    margin: 0 auto;
    border-radius: var(--sk-radius-md);
    background: transparent;
    color: var(--sk-secondary-strong);
    transition: color 150ms ease;
  }

  .mode-icon.active {
    color: var(--sk-text);
  }

  .mode-icon:focus-visible {
    outline: 2px solid var(--sk-data-strong);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .mode-switch::before,
    .collapsed-modes::before {
      transition: none;
    }
  }

  /* ── Badge dot on mode buttons ── */
  .badge-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  .mode-badge {
    width: 8px;
    height: 8px;
    border-radius: var(--sk-radius-pill);
    background: var(--sk-accent);
  }

  .collapsed-modes .mode-badge {
    position: absolute;
    top: -4px;
    right: -4px;
  }

  /* ── Content area — roomier padding, room for expandable sections ── */
  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--sk-space-md) var(--sk-space-sm);
    gap: var(--sk-space-lg);
  }

  /* ── Footer ── */
  .footer {
    padding: var(--sk-space-md) var(--sk-space-lg);
    font-size: var(--sk-font-size-xs);
    color: var(--sk-faded);
    border-top: 1px solid var(--sk-border-lighter);
  }

  .footer-text {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
