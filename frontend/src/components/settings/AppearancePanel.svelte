<script lang="ts">
  import { onMount } from 'svelte';
  import PanelFrame from './PanelFrame.svelte';
  import type { AppearanceSettings } from '../../lib/types';

  let {
    appearance,
    onSave,
  }: {
    appearance: AppearanceSettings;
    onSave: (appearance: AppearanceSettings) => Promise<void>;
  } = $props();

  let dateFormat = $state<'system' | 'YYYY-MM-DD' | 'DD/MM/YYYY' | 'MM/DD/YYYY'>('system');
  let rowDensity = $state<'comfortable' | 'compact'>('comfortable');
  let saving = $state(false);
  let error = $state('');
  let success = $state('');

  onMount(() => {
    dateFormat = appearance.dateFormat;
    rowDensity = appearance.rowDensity;
  });

  async function handleSave() {
    if (saving) {
      return;
    }

    saving = true;
    error = '';
    success = '';
    try {
      await onSave({
        dateFormat,
        rowDensity,
      });
      success = 'Saved';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to save appearance';
    } finally {
      saving = false;
    }
  }
</script>

<PanelFrame
  title="Appearance"
  description="Choose how dates render in the grid and how compact the table workspace should feel."
>
  <div class="grid">
    <div class="card">
      <label for="appearance-date-format">Date format</label>
      <select id="appearance-date-format" bind:value={dateFormat}>
        <option value="system">System default</option>
        <option value="YYYY-MM-DD">YYYY-MM-DD</option>
        <option value="DD/MM/YYYY">DD/MM/YYYY</option>
        <option value="MM/DD/YYYY">MM/DD/YYYY</option>
      </select>

      <label for="appearance-row-density">Row density</label>
      <select id="appearance-row-density" bind:value={rowDensity}>
        <option value="comfortable">Comfortable</option>
        <option value="compact">Compact</option>
      </select>

      <div class="actions">
        <button type="button" class="save" onclick={handleSave} disabled={saving}>
          {saving ? 'Saving…' : 'Save appearance'}
        </button>
        {#if success}
          <span class="hint success">{success}</span>
        {/if}
      </div>

      {#if error}
        <p class="message error">{error}</p>
      {/if}
    </div>

    <div class="card preview">
      <p class="preview-label">What changes now</p>
      <ul>
        <li>Date cells use <strong>{dateFormat}</strong></li>
        <li>Workspace spacing uses <strong>{rowDensity}</strong> density</li>
      </ul>
    </div>
  </div>
</PanelFrame>

<style>
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 280px);
    gap: var(--sk-space-lg);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-md);
    padding: var(--sk-space-xl);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.6);
    box-shadow: var(--sk-shadow-card);
  }

  label {
    font-weight: 500;
  }

  select {
    border: 1px solid rgba(47, 72, 88, 0.14);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    padding: var(--sk-space-sm) var(--sk-space-md);
    color: var(--sk-text);
    font: inherit;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--sk-space-md);
  }

  .save {
    border: none;
    border-radius: var(--sk-radius-md);
    background: var(--sk-accent);
    color: white;
    padding: var(--sk-space-sm) var(--sk-space-lg);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .preview-label,
  .hint {
    color: var(--sk-muted);
  }

  .hint.success {
    color: var(--sk-boolean-true);
  }

  .message.error {
    color: #b91c1c;
  }

  ul {
    padding-left: var(--sk-space-md);
    line-height: 1.6;
  }

  @media (max-width: 900px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
