<script lang="ts">
  import { onMount } from 'svelte';
  import PanelFrame from './PanelFrame.svelte';
  import { checkForUpdates, fetchUpdateStatus } from '../../lib/api';
  import type { UpdateStatusResponse } from '../../lib/types';

  let status = $state<UpdateStatusResponse | null>(null);
  let loading = $state(true);
  let checking = $state(false);
  let error = $state('');

  onMount(async () => {
    try {
      status = await fetchUpdateStatus();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load update status';
    } finally {
      loading = false;
    }
  });

  async function handleCheck() {
    checking = true;
    error = '';
    try {
      status = await checkForUpdates();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to check for updates';
    } finally {
      checking = false;
    }
  }
</script>

<PanelFrame
  title="Updates"
  description="This panel is wired to the shared patcher contract. Until the patcher backend lands on this branch, the actions stay disabled instead of using a fake API."
>
  {#if loading}
    <div class="card muted">Checking update support…</div>
  {:else if error}
    <div class="card error">{error}</div>
  {:else if status}
    <div class="grid">
      <div class="card"><span>Current</span><strong>{status.current}</strong></div>
      <div class="card"><span>Latest</span><strong>{status.latest ?? 'No release found'}</strong></div>
      <div class="card"><span>Channel</span><strong>{status.pre_release_channel ? 'Pre-release' : 'Stable'}</strong></div>
      <div class="card"><span>Rollback</span><strong>{status.previous_exists ? 'Available' : 'Unavailable'}</strong></div>
    </div>

    <div class="actions">
      <button type="button" onclick={handleCheck} disabled={checking}>
        {checking ? 'Checking…' : 'Check for updates'}
      </button>
      <button type="button" disabled>Install update</button>
      <button type="button" disabled>Upload WIP build</button>
      <button type="button" disabled={!status.previous_exists}>Rollback</button>
    </div>
  {:else}
    <div class="card muted">
      <strong>Patcher backend unavailable</strong>
      <p>This branch does not expose `/api/update/*` yet, so the settings page renders the disabled shell instead of inventing a parallel update contract.</p>
    </div>
  {/if}
</PanelFrame>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--sk-space-md);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-xl);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.6);
    box-shadow: var(--sk-shadow-card);
  }

  .card span,
  .card p {
    color: var(--sk-muted);
  }

  .card strong {
    color: var(--sk-text);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-sm);
  }

  button {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-button);
    color: var(--sk-text);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font: inherit;
    cursor: pointer;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .muted {
    color: var(--sk-muted);
  }

  .error {
    color: #b91c1c;
  }
</style>
