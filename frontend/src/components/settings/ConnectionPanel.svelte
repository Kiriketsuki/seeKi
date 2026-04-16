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
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--sk-space-md);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xl);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.6);
    box-shadow: var(--sk-shadow-card);
  }

  .card span {
    color: var(--sk-muted);
    font-size: var(--sk-font-size-body);
  }

  .card strong {
    color: var(--sk-text);
    font-size: var(--sk-font-size-lg);
  }

  .muted {
    color: var(--sk-muted);
  }

  .error {
    color: #b91c1c;
  }
</style>
