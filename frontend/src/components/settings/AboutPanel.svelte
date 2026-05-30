<script lang="ts">
  import { onMount } from 'svelte';
  import PanelFrame from './PanelFrame.svelte';
  import { fetchVersion } from '../../lib/api';
  import type { VersionResponse } from '../../lib/types';

  const REPO_URL = 'https://github.com/Kiriketsuki/seeKi';

  let version = $state<VersionResponse | null>(null);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    try {
      version = await fetchVersion();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load build metadata';
    } finally {
      loading = false;
    }
  });
</script>

<PanelFrame
  title="About"
  description="Build metadata comes from the backend so you can confirm exactly which version is running."
>
  {#if loading}
    <div class="card muted">Loading build metadata…</div>
  {:else if error}
    <div class="card error">{error}</div>
  {:else if version}
    <div class="grid">
      <div class="card"><span>Version</span><strong>{version.version}</strong></div>
      <div class="card"><span>Commit</span><strong>{version.commit}</strong></div>
      <div class="card"><span>Built at</span><strong>{version.built_at}</strong></div>
      <a class="card link" href={REPO_URL} target="_blank" rel="noreferrer">
        <span>Repository</span>
        <strong>{REPO_URL}</strong>
      </a>
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
    background: rgba(255, 255, 255, 0.6);
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
    background: rgba(255, 255, 255, 0.6);
    box-shadow: var(--sk-shadow-card);
  }

  .error {
    color: var(--sk-danger);
    padding: var(--sk-space-xl);
    border: 1px solid rgba(var(--sk-danger-rgb), 0.2);
    border-radius: var(--sk-radius-lg);
    background: rgba(var(--sk-danger-rgb), 0.05);
  }

  /* sk-kv-link hover */
  .link:hover strong {
    color: var(--sk-accent-active-strong);
  }
</style>
