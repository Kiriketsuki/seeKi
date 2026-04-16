<script lang="ts">
  import BrandingPanel from './settings/BrandingPanel.svelte';
  import AppearancePanel from './settings/AppearancePanel.svelte';
  import ConnectionPanel from './settings/ConnectionPanel.svelte';
  import DataPanel from './settings/DataPanel.svelte';
  import AboutPanel from './settings/AboutPanel.svelte';
  import UpdatesPanel from './settings/UpdatesPanel.svelte';
  import { activeSettingsSection } from '../lib/stores';
  import type {
    AppearanceSettings,
    BrandingSettings,
  } from '../lib/types';

  let {
    branding,
    appearance,
    onSaveBranding,
    onSaveAppearance,
  }: {
    branding: BrandingSettings;
    appearance: AppearanceSettings;
    onSaveBranding: (branding: BrandingSettings) => Promise<void>;
    onSaveAppearance: (appearance: AppearanceSettings) => Promise<void>;
  } = $props();
</script>

<div class="settings-content">
  {#key $activeSettingsSection}
    <div class="settings-panel-enter">
      {#if $activeSettingsSection === 'updates'}
        <UpdatesPanel />
      {:else if $activeSettingsSection === 'branding'}
        <BrandingPanel {branding} onSave={onSaveBranding} />
      {:else if $activeSettingsSection === 'appearance'}
        <AppearancePanel {appearance} onSave={onSaveAppearance} />
      {:else if $activeSettingsSection === 'connection'}
        <ConnectionPanel />
      {:else if $activeSettingsSection === 'data'}
        <DataPanel />
      {:else}
        <AboutPanel />
      {/if}
    </div>
  {/key}
</div>

<style>
  .settings-content {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    padding: var(--sk-space-2xl);
    overflow: auto;
  }

  .settings-panel-enter {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    animation: sk-panel-fade 180ms ease-out;
  }
</style>
