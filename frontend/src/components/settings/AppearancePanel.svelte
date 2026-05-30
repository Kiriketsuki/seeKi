<script lang="ts">
  import { onMount } from 'svelte';
  import PanelFrame from './PanelFrame.svelte';
  import type { AppearanceSettings, PalettePreference } from '../../lib/types';

  let {
    appearance,
    onSave,
  }: {
    appearance: AppearanceSettings;
    onSave: (appearance: AppearanceSettings) => Promise<void>;
  } = $props();

  let dateFormat = $state<'system' | 'YYYY-MM-DD' | 'DD/MM/YYYY' | 'MM/DD/YYYY'>('system');
  let rowDensity = $state<'comfortable' | 'compact'>('comfortable');
  let palette = $state<PalettePreference>('alabaster');
  let saving = $state(false);
  let error = $state('');
  let success = $state('');

  const paletteOptions: Array<{ value: PalettePreference; label: string }> = [
    { value: 'alabaster', label: 'Alabaster (default)' },
    { value: 'travertine', label: 'Warm Travertine' },
    { value: 'carrara', label: 'Cool Carrara' },
    { value: 'deepvein', label: 'Deep Vein' },
    { value: 'spa', label: 'Quiet Spa' },
    { value: 'oxide', label: 'Bright Oxide' },
  ];

  onMount(() => {
    dateFormat = appearance.dateFormat;
    rowDensity = appearance.rowDensity;
    palette = appearance.palette ?? 'alabaster';
    // Apply the stored palette immediately so the preview reflects the saved setting
    applyPaletteLive(palette);
  });

  function applyPaletteLive(p: PalettePreference) {
    document.documentElement.setAttribute('data-palette', p);
  }

  function handlePaletteChange(p: PalettePreference) {
    palette = p;
    applyPaletteLive(p);
  }

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
        palette,
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
  description="Choose how dates render in the grid, how compact the table workspace should feel, and the colour palette for the app."
>
  <!-- sk-appear-grid: controls on left, preview on right -->
  <div class="appear-grid">
    <div class="card">
      <label for="appearance-date-format">Date format</label>
      <div class="select-wrap">
        <select id="appearance-date-format" class="select" bind:value={dateFormat}>
          <option value="system">System default</option>
          <option value="YYYY-MM-DD">YYYY-MM-DD</option>
          <option value="DD/MM/YYYY">DD/MM/YYYY</option>
          <option value="MM/DD/YYYY">MM/DD/YYYY</option>
        </select>
        <span class="select-caret" aria-hidden="true">&#8964;</span>
      </div>

      <label for="appearance-row-density">Row density</label>
      <div class="select-wrap">
        <select id="appearance-row-density" class="select" bind:value={rowDensity}>
          <option value="comfortable">Comfortable</option>
          <option value="compact">Compact</option>
        </select>
        <span class="select-caret" aria-hidden="true">&#8964;</span>
      </div>

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
        <li>Date cells use <strong>{dateFormat === 'system' ? 'system default' : dateFormat}</strong></li>
        <li>Workspace spacing uses <strong>{rowDensity}</strong> density</li>
        <li>Palette: <strong>{paletteOptions.find((o) => o.value === palette)?.label ?? palette}</strong></li>
      </ul>
    </div>
  </div>

  <!-- Palette switcher card -->
  <div class="card palette-card">
    <strong class="set-title">Colour palette</strong>
    <p class="set-copy">Changes the marble tones and accent colours for the entire app. Flips live as you select.</p>
    <!-- sk-seg segmented palette control -->
    <div class="palette-seg" role="group" aria-label="Colour palette">
      {#each paletteOptions as opt}
        <button
          type="button"
          class="palette-btn"
          class:active={palette === opt.value}
          onclick={() => handlePaletteChange(opt.value)}
          aria-pressed={palette === opt.value}
          title={opt.label}
        >
          {opt.label}
        </button>
      {/each}
    </div>
  </div>
</PanelFrame>

<style>
  /* sk-appear-grid */
  .appear-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 280px);
    gap: var(--sk-space-lg);
    align-items: start;
  }

  /* sk-set-card */
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-md);
    padding: var(--sk-space-xl);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.6);
    backdrop-filter: var(--sk-glass-grid-blur);
    -webkit-backdrop-filter: var(--sk-glass-grid-blur);
    box-shadow: var(--sk-shadow-card);
  }

  .palette-card {
    gap: var(--sk-space-md);
  }

  label {
    font-weight: 500;
    color: var(--sk-text);
  }

  /* sk-select-wrap */
  .select-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  /* sk-select */
  .select {
    appearance: none;
    -webkit-appearance: none;
    width: 100%;
    border: 1px solid var(--sk-border-input);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    padding: var(--sk-space-sm) calc(var(--sk-space-lg) + 8px) var(--sk-space-sm) var(--sk-space-md);
    color: var(--sk-text);
    font: inherit;
    font-size: var(--sk-font-size-body);
    cursor: pointer;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .select:focus {
    border-color: rgba(var(--marble-active-rgb), 0.45);
    box-shadow: 0 0 0 2px var(--sk-ring);
  }

  .select-caret {
    position: absolute;
    right: var(--sk-space-sm);
    color: var(--sk-ink-faint);
    pointer-events: none;
    font-size: 14px;
  }

  /* sk-set-actions */
  .actions {
    display: flex;
    align-items: center;
    gap: var(--sk-space-md);
  }

  /* sk-btn-accent — amber CTA */
  .save {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-sm);
    border: none;
    border-radius: var(--sk-radius-md);
    background: var(--sk-accent);
    color: var(--sk-on-accent);
    box-shadow: var(--sk-shadow-accent);
    padding: var(--sk-space-sm) var(--sk-space-lg);
    font: inherit;
    font-size: var(--sk-font-size-body);
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s ease, box-shadow 0.15s ease;
  }

  .save:hover:not(:disabled) {
    opacity: 0.93;
  }

  .save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }

  /* sk-preview-label */
  .preview-label {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }

  .hint {
    color: var(--sk-muted);
    font-size: var(--sk-font-size-body);
  }

  .hint.success {
    color: var(--sk-boolean-true);
  }

  .message.error {
    color: var(--sk-danger);
    margin: 0;
  }

  ul {
    padding-left: var(--sk-space-md);
    line-height: 1.8;
    color: var(--sk-secondary-strong);
    margin: 0;
  }

  /* sk-set-title + sk-set-copy */
  .set-title {
    font-weight: 600;
    color: var(--sk-text);
  }

  .set-copy {
    color: var(--sk-secondary-strong);
    line-height: 1.5;
    max-width: 50ch;
    margin: 0;
  }

  /* Palette segmented control — sk-seg variant */
  .palette-seg {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xs);
    background: rgba(var(--marble-vein-rgb), 0.05);
    border-radius: var(--sk-radius-md);
  }

  .palette-btn {
    border: none;
    background: transparent;
    color: var(--sk-secondary-strong);
    padding: var(--sk-space-xs) var(--sk-space-md);
    border-radius: var(--sk-radius-sm);
    font: inherit;
    font-size: var(--sk-font-size-body);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    white-space: nowrap;
  }

  /* sk-seg-btn.active — lifted glass pill, ink text */
  .palette-btn.active {
    background: var(--sk-glass-button);
    color: var(--sk-text);
    box-shadow: 0 1px 3px rgba(var(--marble-vein-rgb), 0.1);
    font-weight: 500;
  }

  .palette-btn:not(.active):hover {
    background: rgba(255, 255, 255, 0.5);
    color: var(--sk-text);
  }

  .palette-btn:focus-visible {
    outline: 2px solid var(--sk-data-strong);
    outline-offset: 2px;
  }

  @media (max-width: 1100px) {
    .appear-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
