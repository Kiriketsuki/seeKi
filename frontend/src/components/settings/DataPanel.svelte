<script lang="ts">
  import PanelFrame from './PanelFrame.svelte';
  import { clearAllPresets } from '../../lib/api';
  import { COLUMN_VISIBILITY_KEY_PREFIX, SIDEBAR_COLLAPSED_KEY } from '../../lib/constants';

  let clearing = $state(false);
  let cleared = $state(false);
  let error = $state('');

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
</style>
