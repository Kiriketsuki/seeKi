<script lang="ts">
  import { activeSettingsSection } from '../lib/stores';
  import type { SettingsSection } from '../lib/types';

  let {
    showUpdateBadge = false,
  }: {
    showUpdateBadge?: boolean;
  } = $props();

  const sections: Array<{ id: SettingsSection; label: string }> = [
    { id: 'updates', label: 'Updates' },
    { id: 'branding', label: 'Branding' },
    { id: 'appearance', label: 'Appearance' },
    { id: 'connection', label: 'Connection' },
    { id: 'data', label: 'Data' },
    { id: 'about', label: 'About' },
  ];
</script>

<nav class="settings-nav" aria-label="Settings sections">
  {#each sections as section}
    <button
      type="button"
      class="nav-item"
      class:active={$activeSettingsSection === section.id}
      onclick={() => activeSettingsSection.set(section.id)}
    >
      <span>{section.label}</span>
      {#if section.id === 'updates' && showUpdateBadge}
        <span class="badge" aria-label="Update available"></span>
      {/if}
    </button>
  {/each}
</nav>

<style>
  .settings-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--sk-space-xs) 0;
  }

  .nav-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    width: 100%;
    border: none;
    border-radius: var(--sk-radius-sm);
    background: none;
    color: var(--sk-text);
    padding: var(--sk-space-xs) var(--sk-space-md);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    text-align: left;
    cursor: pointer;
  }

  .nav-item:hover {
    background: var(--sk-border);
  }

  .nav-item.active {
    background: var(--sk-accent);
    color: white;
  }

  .badge {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--sk-accent);
    flex-shrink: 0;
  }

  .nav-item.active .badge {
    background: white;
  }
</style>
