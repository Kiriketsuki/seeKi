<script lang="ts">
  import BrandingPanel from './settings/BrandingPanel.svelte';
  import AppearancePanel from './settings/AppearancePanel.svelte';
  import ConnectionPanel from './settings/ConnectionPanel.svelte';
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
  {#if $activeSettingsSection === 'updates'}
    <UpdatesPanel />
  {:else if $activeSettingsSection === 'branding'}
    <BrandingPanel {branding} onSave={onSaveBranding} />
  {:else if $activeSettingsSection === 'appearance'}
    <AppearancePanel {appearance} onSave={onSaveAppearance} />
  {:else if $activeSettingsSection === 'connection'}
    <ConnectionPanel />
  {:else}
    <AboutPanel />
  {/if}
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
</style>
