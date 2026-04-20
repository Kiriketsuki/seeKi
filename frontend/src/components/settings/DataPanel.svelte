<script lang="ts">
  import PanelFrame from './PanelFrame.svelte';
  import { clearAllPresets, saveSettings } from '../../lib/api';
  import { COLUMN_VISIBILITY_KEY_PREFIX, SIDEBAR_COLLAPSED_KEY } from '../../lib/constants';
  import { buildDataSettingsEntries } from '../../lib/settings';
  import type { PaginationMode } from '../../lib/types';

  let {
    paginationMode = 'infinite',
    onPaginationModeChange,
  }: {
    paginationMode?: PaginationMode;
    onPaginationModeChange?: (mode: PaginationMode) => void;
  } = $props();

  let clearing = $state(false);
  let cleared = $state(false);
  let error = $state('');
  let savingMode = $state(false);

  async function handleModeChange(mode: PaginationMode) {
    if (savingMode || mode === paginationMode) return;
    savingMode = true;
    try {
      const entries = buildDataSettingsEntries({ pageSize: 50, paginationMode: mode });
      await saveSettings(entries);
      onPaginationModeChange?.(mode);
    } catch {
      // Non-fatal — surface no error, the parent still applies the mode change.
      onPaginationModeChange?.(mode);
    } finally {
      savingMode = false;
    }
  }

  function clearLocalStorage() {
    if (typeof localStorage === 'undefined') return;
    const keysToRemove: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key && (key.startsWith(COLUMN_VISIBILITY_KEY_PREFIX) || key === SIDEBAR_COLLAPSED_KEY)) {
        keysToRemove.push(key);
      }
    }
    for (const key of keysToRemove) {
      localStorage.removeItem(key);
    }
  }

  async function handleClear() {
    if (clearing) return;
    clearing = true;
    cleared = false;
    error = '';
    try {
      await clearAllPresets();
      clearLocalStorage();
      cleared = true;
      setTimeout(() => { cleared = false; }, 3000);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to clear data';
    } finally {
      clearing = false;
    }
  }
</script>

<PanelFrame
  title="Data"
  description="Manage the browsing state SeeKi stores locally — remembered sort orders, filters, search terms, column visibility, and presets."
>
  <div class="card">
    <div class="row">
      <div class="info">
        <strong>Browsing mode</strong>
        <p>Infinite scroll loads more rows as you scroll. Paged browsing uses Previous / Next controls and loads one page at a time.</p>
      </div>
      <div class="action">
        <div class="mode-toggle" role="group" aria-label="Browsing mode">
          <button
            type="button"
            class="mode-btn"
            class:active={paginationMode === 'infinite'}
            disabled={savingMode}
            onclick={() => void handleModeChange('infinite')}
          >
            Infinite scroll
          </button>
          <button
            type="button"
            class="mode-btn"
            class:active={paginationMode === 'paged'}
            disabled={savingMode}
            onclick={() => void handleModeChange('paged')}
          >
            Paged
          </button>
        </div>
      </div>
    </div>
  </div>

  <div class="card">
    <div class="row">
      <div class="info">
        <strong>Clear browsing data</strong>
        <p>Removes remembered sort orders, filters, search terms, column visibility, and saved presets. Saved views stay intact. Configuration (branding, appearance) is not affected.</p>
      </div>
      <div class="action">
        <button type="button" class="danger" onclick={handleClear} disabled={clearing}>
          {clearing ? 'Clearing…' : 'Clear all data'}
        </button>
        {#if cleared}
          <span class="hint success">Cleared</span>
        {/if}
      </div>
    </div>
    {#if error}
      <p class="message error">{error}</p>
    {/if}
  </div>
</PanelFrame>

<style>
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

  .row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-xl);
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
  }

  .info p {
    color: var(--sk-secondary-strong);
    line-height: 1.5;
    max-width: 48ch;
  }

  .action {
    display: flex;
    align-items: center;
    gap: var(--sk-space-md);
    flex-shrink: 0;
  }

  .danger {
    border: 1px solid rgba(185, 28, 28, 0.3);
    border-radius: var(--sk-radius-md);
    background: rgba(185, 28, 28, 0.08);
    color: #b91c1c;
    padding: var(--sk-space-sm) var(--sk-space-lg);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .danger:hover:not(:disabled) {
    background: rgba(185, 28, 28, 0.14);
  }

  .danger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .hint.success {
    color: var(--sk-boolean-true);
  }

  .message.error {
    color: #b91c1c;
  }

  .mode-toggle {
    display: flex;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    overflow: hidden;
  }

  .mode-btn {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--sk-secondary-strong);
    font: inherit;
    padding: var(--sk-space-sm) var(--sk-space-md);
    cursor: pointer;
    white-space: nowrap;
    transition: background 100ms;
  }

  .mode-btn + .mode-btn {
    border-left: 1px solid var(--sk-border-light);
  }

  .mode-btn.active {
    background: var(--sk-accent);
    color: #fff;
    font-weight: 600;
  }

  .mode-btn:not(.active):hover:not(:disabled) {
    background: rgba(47, 72, 88, 0.05);
  }

  .mode-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
