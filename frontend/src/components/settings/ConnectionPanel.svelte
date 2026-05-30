<script lang="ts">
  import { onMount } from 'svelte';
  import PanelFrame from './PanelFrame.svelte';
  import { fetchConnectionStatus } from '../../lib/api';
  import type { ConnectionStatusResponse } from '../../lib/types';

  let status = $state<ConnectionStatusResponse | null>(null);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    try {
      status = await fetchConnectionStatus();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load connection details';
    } finally {
      loading = false;
    }
  });
</script>

<PanelFrame
  title="Connection"
  description="See the live runtime connection details without exposing credentials or editing the loaded config."
>
  {#if loading}
    <div class="card muted">Loading connection details…</div>
  {:else if error}
    <div class="card error">{error}</div>
  {:else if status}
    <div class="grid">
      <div class="card"><span>Database kind</span><strong>{status.database_kind}</strong></div>
      <div class="card"><span>Host</span><strong>{status.host ?? '—'}</strong></div>
      <div class="card"><span>Port</span><strong>{status.port ?? '—'}</strong></div>
      <div class="card"><span>Database</span><strong>{status.database ?? '—'}</strong></div>
      <div class="card"><span>Schemas</span><strong>{status.schemas.join(', ')}</strong></div>
      <div class="card"><span>SSH</span><strong>{status.ssh_enabled ? (status.ssh_connected ? 'Configured and connected' : 'Configured') : 'Not configured'}</strong></div>
    </div>
  {/if}
</PanelFrame>

<style>
  /* sk-conn-grid */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--sk-space-md);
  }

  /* sk-set-card sk-kv */
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-lg) var(--sk-space-xl);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: var(--sk-glass-input);
    backdrop-filter: var(--sk-glass-grid-blur);
    -webkit-backdrop-filter: var(--sk-glass-grid-blur);
    box-shadow: var(--sk-shadow-card);
    text-decoration: none;
  }

  /* sk-kv span */
  .card span {
    color: var(--sk-muted);
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--sk-font-size-body);
  }

  /* sk-kv strong */
  .card strong {
    color: var(--sk-text);
    font-size: var(--sk-font-size-lg);
    overflow-wrap: anywhere;
  }

  .muted {
    color: var(--sk-muted);
    padding: var(--sk-space-xl);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: var(--sk-glass-input);
    box-shadow: var(--sk-shadow-card);
  }

  .error {
    color: var(--sk-danger);
    padding: var(--sk-space-xl);
    border: 1px solid rgba(var(--sk-danger-rgb), 0.2);
    border-radius: var(--sk-radius-lg);
    background: rgba(var(--sk-danger-rgb), 0.05);
  }
</style>
