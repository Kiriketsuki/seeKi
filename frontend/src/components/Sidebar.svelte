<script lang="ts">
  import { type Snippet } from 'svelte';
  import {
    ChevronLeft,
    ChevronRight,
    LayoutGrid,
    Settings,
  } from 'lucide-svelte';
  import { SIDEBAR_COLLAPSED_KEY } from '../lib/constants';
  import SettingsGear from './SettingsGear.svelte';
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

<aside class="sidebar" class:collapsed>
  <div class="header">
    {#if !collapsed}
      <div class="branding">
        <div class="title-row">
          <img class="mark" src="/logo-mark.svg" alt="" aria-hidden="true" width="20" height="20" />
          <span class="title">{title}</span>
        </div>
        {#if subtitle}
          <span class="subtitle">{subtitle}</span>
        {/if}
      </div>
    {:else}
      <img class="mark mark-collapsed" src="/logo-mark.svg" alt="SeeKi" width="20" height="20" />
    {/if}

    <button class="toggle" onclick={handleToggle} aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}>
      {#if collapsed}
        <ChevronRight size={16} />
      {:else}
        <ChevronLeft size={16} />
      {/if}
    </button>
  </div>

  {#if showModeSwitch}
    {#if collapsed}
      <div class="collapsed-modes" aria-label="Workspace mode">
        <button
          type="button"
          class="mode-icon"
          class:active={mode === 'tables'}
          aria-label="Show tables workspace"
          onclick={() => selectMode('tables')}
        >
          <LayoutGrid size={16} />
        </button>
        <button
          type="button"
          class="mode-icon"
          class:active={mode === 'settings'}
          aria-label="Show settings workspace"
          onclick={() => selectMode('settings')}
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
      <div class="mode-switch" role="tablist" aria-label="Workspace mode">
        <button
          type="button"
          class="mode-button"
          class:active={mode === 'tables'}
          onclick={() => selectMode('tables')}
        >
          Tables
        </button>
        <button
          type="button"
          class="mode-button"
          class:active={mode === 'settings'}
          onclick={() => selectMode('settings')}
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
      <SettingsGear {updateAvailable} onclick={onSettingsClick} />
    </div>
  {:else}
    <div class="footer footer-collapsed">
      <SettingsGear {updateAvailable} onclick={onSettingsClick} />
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    width: var(--sk-sidebar-width);
    min-width: var(--sk-sidebar-width);
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--sk-glass-sidebar);
    backdrop-filter: var(--sk-glass-sidebar-blur);
    -webkit-backdrop-filter: var(--sk-glass-sidebar-blur);
    border-right: 1px solid var(--sk-border);
    transition: width 0.2s ease, min-width 0.2s ease;
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: var(--sk-sidebar-collapsed);
    min-width: var(--sk-sidebar-collapsed);
  }

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
    gap: 2px;
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

  .toggle {
    width: 24px;
    height: 24px;
    background: none;
    color: var(--sk-muted);
    border-radius: var(--sk-radius-sm);
    flex-shrink: 0;
  }

  .toggle:hover {
    background: var(--sk-border);
    color: var(--sk-text);
  }

  .mode-switch {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--sk-space-xs);
    padding: var(--sk-space-sm);
    border-bottom: 1px solid var(--sk-border-light);
  }

  .mode-button {
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.55);
    color: var(--sk-secondary-strong);
    padding: var(--sk-space-sm);
    font: inherit;
    font-weight: 500;
  }

  .mode-button.active {
    background: rgba(255, 149, 0, 0.16);
    color: var(--sk-text);
  }

  .collapsed-modes {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-sm) 0;
    border-bottom: 1px solid var(--sk-border-light);
  }

  .mode-icon {
    width: 32px;
    height: 32px;
    margin: 0 auto;
    border-radius: var(--sk-radius-md);
    background: transparent;
    color: var(--sk-secondary-strong);
  }

  .mode-icon.active {
    background: rgba(255, 149, 0, 0.16);
    color: var(--sk-text);
  }

  .badge-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .mode-badge {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--sk-accent);
  }

  .collapsed-modes .mode-badge {
    position: absolute;
    top: -4px;
    right: -4px;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: var(--sk-space-sm);
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-space-md) var(--sk-space-lg);
    font-size: var(--sk-font-size-xs);
    color: var(--sk-faded);
    border-top: 1px solid var(--sk-border-lighter);
    white-space: nowrap;
    gap: var(--sk-space-sm);
  }

  .footer-text {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .footer-collapsed {
    justify-content: center;
    padding: var(--sk-space-md);
  }
</style>
