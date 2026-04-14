<script lang="ts">
  import { type Snippet } from 'svelte';
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { SIDEBAR_COLLAPSED_KEY } from '../lib/constants';
  import SettingsGear from './SettingsGear.svelte';


  let {
    collapsed = $bindable(false),
    onToggle,
    title = 'SeeKi',
    subtitle = '',
    updateAvailable = false,
    onSettingsClick = () => {},
    children,
  }: {
    collapsed: boolean;
    onToggle: () => void;
    title: string;
    subtitle: string;
    updateAvailable?: boolean;
    onSettingsClick?: () => void;
    children?: Snippet;
  } = $props();

  // localStorage restore is handled by the parent (App.svelte) reading SIDEBAR_COLLAPSED_KEY at init

  function handleToggle() {
    const nextCollapsed = !collapsed;
    onToggle();
    localStorage.setItem(SIDEBAR_COLLAPSED_KEY, String(nextCollapsed));
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

  .toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    background: none;
    color: var(--sk-muted);
    cursor: pointer;
    border-radius: var(--sk-radius-sm);
    flex-shrink: 0;
  }

  .toggle:hover {
    background: var(--sk-border);
    color: var(--sk-text);
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
