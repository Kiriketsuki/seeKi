<script lang="ts">
  import { type Snippet } from 'svelte';
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';

  const STORAGE_KEY = 'sk-sidebar-collapsed';

  let {
    collapsed = $bindable(false),
    onToggle,
    title = 'SeeKi',
    subtitle = '',
    children,
  }: {
    collapsed: boolean;
    onToggle: () => void;
    title: string;
    subtitle: string;
    children?: Snippet;
  } = $props();

  // localStorage restore is handled by the parent initializing collapsed from getInitialCollapsed()

  function handleToggle() {
    const nextCollapsed = !collapsed;
    onToggle();
    localStorage.setItem(STORAGE_KEY, String(nextCollapsed));
  }
</script>

<aside class="sidebar" class:collapsed>
  <div class="header">
    {#if !collapsed}
      <div class="branding">
        <span class="title">{title}</span>
        {#if subtitle}
          <span class="subtitle">{subtitle}</span>
        {/if}
      </div>
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
      Powered by SeeKi
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
    padding: var(--sk-space-md) var(--sk-space-lg);
    font-size: var(--sk-font-size-xs);
    color: var(--sk-faded);
    border-top: 1px solid var(--sk-border-lighter);
    white-space: nowrap;
  }
</style>
