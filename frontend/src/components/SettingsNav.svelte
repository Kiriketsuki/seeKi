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
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xs) 0;
  }

  /* roomier rows: 7px padding, radius-md, marble-keyed hover */
  .nav-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    width: 100%;
    border: none;
    border-radius: var(--sk-radius-md);
    background: none;
    color: var(--sk-text);
    padding: 7px var(--sk-space-md);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    text-align: left;
    cursor: pointer;
    transition: background 0.12s ease;
  }

  /* hover: teal soft wash */
  .nav-item:hover {
    background: var(--sk-active-tint-soft);
  }

  /* active-soft: teal tint (not amber — settings nav uses the "active" teal token) */
  .nav-item.active {
    background: rgba(var(--marble-active-rgb), 0.1);
    color: var(--sk-text);
    box-shadow: inset 2px 0 0 var(--sk-accent-active);
  }

  /* update badge dot */
  .badge {
    width: 8px;
    height: 8px;
    border-radius: var(--sk-radius-pill);
    background: var(--sk-accent-count);
    flex-shrink: 0;
  }
</style>
