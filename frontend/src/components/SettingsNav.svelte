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
    gap: var(--sk-space-xs);
  }

  .nav-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    width: 100%;
    border: 1px solid transparent;
    border-radius: var(--sk-radius-md);
    background: transparent;
    color: var(--sk-text);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .nav-item:hover,
  .nav-item.active {
    background: rgba(255, 149, 0, 0.12);
    border-color: rgba(255, 149, 0, 0.18);
  }

  .badge {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--sk-accent);
    flex-shrink: 0;
  }
</style>
