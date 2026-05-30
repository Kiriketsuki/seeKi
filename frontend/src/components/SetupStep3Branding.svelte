<script lang="ts">
  import type { WizardData } from '../lib/types';

  let {
    wizardData = $bindable(),
    onNext,
    onBack,
  }: {
    wizardData: WizardData;
    onNext: () => void;
    onBack: () => void;
  } = $props();

  let titleEmpty = $derived(wizardData.title.trim().length === 0);
</script>

<div class="step">
  <div class="layout">
    <!-- Form fields -->
    <div class="fields">
      <div class="field">
        <label for="brand-title">
          App Title <span class="required" aria-label="required">*</span>
        </label>
        <input
          id="brand-title"
          type="text"
          placeholder="My Database"
          bind:value={wizardData.title}
          aria-required="true"
          aria-invalid={titleEmpty}
        />
      </div>
      <div class="field">
        <label for="brand-subtitle">
          Subtitle <span class="optional">(optional)</span>
        </label>
        <input
          id="brand-subtitle"
          type="text"
          placeholder="Powered by SeeKi"
          bind:value={wizardData.subtitle}
        />
      </div>
    </div>

    <!-- Live preview -->
    <div class="preview-wrapper">
      <p class="preview-label">Preview</p>
      <div class="preview-sidebar">
        <div class="preview-header">
          <div class="preview-branding">
            <span class="preview-title">{wizardData.title || 'App Title'}</span>
            {#if wizardData.subtitle}
              <span class="preview-subtitle">{wizardData.subtitle}</span>
            {:else}
              <span class="preview-subtitle placeholder">Subtitle</span>
            {/if}
          </div>
          <div class="preview-toggle-btn"></div>
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

  <!-- Actions -->
  <div class="actions">
    <button class="btn-back" onclick={onBack} aria-label="Go back to table selection">← Back</button>
    <div class="right-actions">
      {#if titleEmpty}
        <span class="hint">Title is required</span>
      {/if}
      <button
        class="btn-next"
        onclick={onNext}
        disabled={titleEmpty}
        aria-label="Proceed to confirmation"
      >
        Next →
      </button>
    </div>
  </div>
</div>

<style>
  .step { display: flex; flex-direction: column; gap: var(--sk-space-xl); }

  .layout {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--sk-space-xl);
    align-items: start;
  }

  @media (max-width: 560px) {
    .layout { grid-template-columns: 1fr; }
    .preview-wrapper { order: -1; }
  }

  .fields { display: flex; flex-direction: column; gap: var(--sk-space-lg); }

  /* sk-field */
  .field { display: flex; flex-direction: column; gap: var(--sk-space-xs); }

  .field label {
    font-size: var(--sk-font-size-body);
    font-weight: 500;
    color: var(--sk-text);
  }

  .required { color: var(--sk-danger); }
  .optional { font-weight: 400; color: var(--sk-muted); }

  .field input {
    background: var(--sk-glass-input);
    border: 1px solid var(--sk-border-input);
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .field input:focus {
    border-color: rgba(var(--marble-active-rgb), 0.45);
    box-shadow: 0 0 0 3px var(--sk-ring);
  }

  .field input[aria-invalid="true"] {
    border-color: rgba(185, 28, 28, 0.4);
  }

  /* Preview sidebar */
  .preview-wrapper { display: flex; flex-direction: column; gap: var(--sk-space-sm); }

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
    width: 160px;
    min-height: 200px;
    background: var(--sk-glass-sidebar);
    backdrop-filter: var(--sk-glass-sidebar-blur);
    -webkit-backdrop-filter: var(--sk-glass-sidebar-blur);
    border: 1px solid var(--sk-border);
    border-radius: var(--sk-radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--sk-shadow-card);
  }

  /* sk-mini-head */
  .preview-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-space-md);
    border-bottom: 1px solid var(--sk-border-light);
  }

  .preview-branding { display: flex; flex-direction: column; gap: var(--sk-space-xs); min-width: 0; }

  /* sk-mini-title */
  .preview-title {
    font-size: var(--sk-font-size-body);
    font-weight: 600;
    color: var(--sk-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 110px;
  }

  /* sk-mini-sub */
  .preview-subtitle {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 110px;
  }

  .preview-subtitle.placeholder { color: var(--sk-faded); font-style: italic; }

  /* sk-mini-toggle */
  .preview-toggle-btn {
    width: 16px;
    height: 16px;
    border-radius: var(--sk-radius-sm);
    background: var(--sk-border);
    flex-shrink: 0;
  }

  /* sk-mini-content */
  .preview-content {
    flex: 1;
    padding: var(--sk-space-sm);
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
  }

  /* sk-mini-item */
  .preview-item {
    height: 24px;
    border-radius: var(--sk-radius-sm);
    background: rgba(var(--marble-vein-rgb), 0.05);
  }

  /* sk-mini-item.active — amber inset bar */
  .preview-item.active {
    box-shadow: inset 2px 0 0 var(--sk-accent-count);
    background: rgba(var(--marble-count-rgb), 0.14);
  }

  /* sk-mini-footer */
  .preview-footer {
    padding: var(--sk-space-sm) var(--sk-space-md);
    font-size: var(--sk-font-size-xs);
    color: var(--sk-faded);
    border-top: 1px solid var(--sk-border-lighter);
  }

  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .right-actions { display: flex; align-items: center; gap: var(--sk-space-md); }
  .hint { font-size: var(--sk-font-size-sm); color: var(--sk-muted); }

  /* sk-btn-test (ghost back button) */
  .btn-back {
    padding: var(--sk-space-sm) var(--sk-space-lg);
    background: var(--sk-glass-button);
    border: 1px solid var(--sk-border-input);
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, box-shadow 0.15s;
  }

  .btn-back:hover {
    background: #fff;
    border-color: rgba(var(--marble-active-rgb), 0.24);
    box-shadow: var(--sk-shadow-card);
  }

  /* sk-btn-next — amber CTA */
  .btn-next {
    padding: var(--sk-space-sm) var(--sk-space-2xl);
    background: var(--sk-accent);
    color: #fff;
    border: none;
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-md);
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s ease, box-shadow 0.15s ease;
    box-shadow: var(--sk-shadow-accent);
  }

  .btn-next:hover:not(:disabled) { opacity: 0.92; }
  .btn-next:disabled { opacity: 0.45; cursor: not-allowed; box-shadow: none; }
</style>
