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
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(220px, 320px);
    gap: var(--sk-space-lg);
    align-items: start;
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

  .fields label {
    font-weight: 500;
    color: var(--sk-text);
  }

  .fields input {
    border: 1px solid rgba(47, 72, 88, 0.14);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    padding: var(--sk-space-sm) var(--sk-space-md);
    color: var(--sk-text);
    font: inherit;
  }

  .fields input[aria-invalid="true"] {
    border-color: rgba(220, 38, 38, 0.4);
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
    cursor: not-allowed;
    opacity: 0.5;
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
    color: #b91c1c;
  }

  .preview-label {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .preview-sidebar {
    width: 100%;
    min-height: 220px;
    background: var(--sk-glass-sidebar);
    border: 1px solid var(--sk-border);
    border-radius: var(--sk-radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .preview-header,
  .preview-footer {
    border-bottom: 1px solid var(--sk-border-light);
    padding: var(--sk-space-md);
  }

  .preview-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .preview-branding {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .preview-title,
  .preview-subtitle {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .preview-title {
    font-weight: 600;
  }

  .preview-subtitle {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
  }

  .placeholder {
    font-style: italic;
    color: var(--sk-faded);
  }

  .preview-toggle {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    background: var(--sk-border);
  }

  .preview-content {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 6px;
    padding: var(--sk-space-md);
  }

  .preview-item {
    height: 28px;
    border-radius: 4px;
    background: rgba(47, 72, 88, 0.06);
  }

  .preview-item.active {
    border-left: 2px solid var(--sk-accent);
    background: rgba(255, 149, 0, 0.12);
  }

  .preview-footer {
    margin-top: auto;
    border-top: 1px solid var(--sk-border-light);
    border-bottom: none;
    color: var(--sk-faded);
    font-size: var(--sk-font-size-sm);
  }

  @media (max-width: 900px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
</style>
