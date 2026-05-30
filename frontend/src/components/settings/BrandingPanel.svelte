<script lang="ts">
  import { onMount } from 'svelte';
  import PanelFrame from './PanelFrame.svelte';
  import type { BrandingSettings } from '../../lib/types';

  let {
    branding,
    onSave,
  }: {
    branding: BrandingSettings;
    onSave: (branding: BrandingSettings) => Promise<void>;
  } = $props();

  let title = $state('');
  let subtitle = $state('');
  let saving = $state(false);
  let error = $state('');
  let success = $state('');
  let titleEmpty = $derived(title.trim().length === 0);

  onMount(() => {
    title = branding.title;
    subtitle = branding.subtitle;
  });

  async function handleSave() {
    if (titleEmpty || saving) {
      return;
    }

    saving = true;
    error = '';
    success = '';
    try {
      await onSave({
        title: title.trim(),
        subtitle: subtitle.trim(),
      });
      success = 'Saved';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to save branding';
    } finally {
      saving = false;
    }
  }
</script>

<PanelFrame
  title="Branding"
  description="Update the app title and subtitle shown in the sidebar. Changes are saved to local settings and become the new default after refresh."
>
  <div class="layout">
    <div class="card fields">
      <label for="settings-brand-title">
        App Title <span class="required">*</span>
      </label>
      <input
        id="settings-brand-title"
        type="text"
        bind:value={title}
        aria-invalid={titleEmpty}
        placeholder="My Database"
      />

      <label for="settings-brand-subtitle">Subtitle</label>
      <input
        id="settings-brand-subtitle"
        type="text"
        bind:value={subtitle}
        placeholder="Powered by SeeKi"
      />

      <div class="actions">
        <button type="button" class="save" onclick={handleSave} disabled={titleEmpty || saving}>
          {saving ? 'Saving…' : 'Save branding'}
        </button>
        {#if titleEmpty}
          <span class="hint">Title is required</span>
        {:else if success}
          <span class="hint success">{success}</span>
        {/if}
      </div>

      {#if error}
        <p class="message error">{error}</p>
      {/if}
    </div>

    <div class="card preview">
      <p class="preview-label">Preview</p>
      <div class="preview-sidebar">
        <div class="preview-header">
          <div class="preview-branding">
            <span class="preview-title">{title || 'App Title'}</span>
            {#if subtitle}
              <span class="preview-subtitle">{subtitle}</span>
            {:else}
              <span class="preview-subtitle placeholder">Subtitle</span>
            {/if}
          </div>
          <div class="preview-toggle"></div>
        </div>
        <div class="preview-content">
          <div class="preview-item active"></div>
          <div class="preview-item"></div>
          <div class="preview-item"></div>
        </div>
        <div class="preview-footer">Powered by SeeKi</div>
      </div>
    </div>
  </div>
</PanelFrame>

<style>
  /* sk-brand-layout */
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(220px, 320px);
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

  .fields label {
    font-weight: 500;
    color: var(--sk-text);
  }

  .fields input {
    border: 1px solid var(--sk-border-input);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    padding: var(--sk-space-sm) var(--sk-space-md);
    color: var(--sk-text);
    font: inherit;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .fields input:focus {
    border-color: rgba(var(--marble-active-rgb), 0.45);
    box-shadow: 0 0 0 2px var(--sk-ring);
  }

  .fields input[aria-invalid="true"] {
    border-color: rgba(var(--sk-danger-rgb), 0.4);
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
    cursor: not-allowed;
    opacity: 0.5;
    box-shadow: none;
  }

  .hint {
    color: var(--sk-muted);
    font-size: var(--sk-font-size-body);
  }

  .hint.success {
    color: var(--sk-boolean-true);
  }

  .message.error,
  .required {
    color: var(--sk-danger);
  }

  /* sk-preview-label */
  .preview-label {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }

  /* sk-mini-sidebar */
  .preview-sidebar {
    width: 100%;
    min-height: 200px;
    background: var(--sk-glass-sidebar);
    backdrop-filter: var(--sk-glass-sidebar-blur);
    -webkit-backdrop-filter: var(--sk-glass-sidebar-blur);
    border: 1px solid var(--sk-border);
    border-radius: var(--sk-radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* sk-mini-head */
  .preview-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-space-md);
    border-bottom: 1px solid var(--sk-border-light);
  }

  /* sk-mini-brand */
  .preview-branding {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    min-width: 0;
  }

  /* sk-mini-title */
  .preview-title {
    font-weight: 600;
    color: var(--sk-text);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  /* sk-mini-sub */
  .preview-subtitle {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .placeholder {
    font-style: italic;
    color: var(--sk-faded);
  }

  /* sk-mini-toggle */
  .preview-toggle {
    width: 18px;
    height: 18px;
    border-radius: var(--sk-radius-sm);
    background: var(--sk-border);
    flex-shrink: 0;
  }

  /* sk-mini-content */
  .preview-content {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-md);
  }

  /* sk-mini-item */
  .preview-item {
    height: 26px;
    border-radius: var(--sk-radius-sm);
    background: rgba(var(--marble-vein-rgb), 0.06);
  }

  /* sk-mini-item.active — amber left-border accent */
  .preview-item.active {
    box-shadow: inset 2px 0 0 var(--sk-accent-count);
    background: rgba(var(--marble-count-rgb), 0.14);
  }

  /* sk-mini-footer */
  .preview-footer {
    margin-top: auto;
    border-top: 1px solid var(--sk-border-light);
    padding: var(--sk-space-md);
    color: var(--sk-faded);
    font-size: var(--sk-font-size-sm);
  }

  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
</style>
