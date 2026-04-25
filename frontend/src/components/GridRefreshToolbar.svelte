<script lang="ts">
  import { RefreshCw } from 'lucide-svelte';
  import { GRID_REFRESH_INTERVALS } from '../lib/constants';
  import type { GridRefreshInterval } from '../lib/stores';

  let {
    surfaceKey = '',
    intervalMs = 0,
    refreshing = false,
    disabled = false,
    onRefreshNow = () => {},
    onIntervalChange = () => {},
  }: {
    surfaceKey?: string;
    intervalMs?: GridRefreshInterval;
    refreshing?: boolean;
    disabled?: boolean;
    onRefreshNow?: () => void;
    onIntervalChange?: (intervalMs: GridRefreshInterval) => void;
  } = $props();

  const intervalOptions = GRID_REFRESH_INTERVALS.map((value) => ({
    value,
    label:
      value === 0 ? 'Off' : value === 15_000 ? '15s' : value === 60_000 ? '1m' : '5m',
  }));
</script>

<div
  class="grid-refresh-toolbar"
  class:is-disabled={disabled}
  data-testid="grid-refresh-toolbar"
>
  <button
    type="button"
    class="refresh-button"
    onclick={onRefreshNow}
    disabled={disabled || refreshing || !surfaceKey}
    data-testid="grid-refresh-now"
  >
    <span class:spin={refreshing}>
      <RefreshCw size={14} />
    </span>
    <span>{refreshing ? 'Refreshing…' : 'Refresh now'}</span>
  </button>

  <div class="refresh-intervals" role="group" aria-label="Auto-refresh interval">
    <span class="refresh-label">Auto-refresh</span>
    <div class="refresh-pills" data-testid="grid-refresh-intervals">
      {#each intervalOptions as option}
        <button
          type="button"
          class="refresh-pill"
          class:is-active={intervalMs === option.value}
          onclick={() => onIntervalChange(option.value)}
          disabled={disabled || !surfaceKey}
          data-testid={`grid-refresh-interval-${option.label.toLowerCase()}`}
        >
          {option.label}
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .grid-refresh-toolbar {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    color: var(--sk-muted);
  }

  .refresh-button,
  .refresh-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-sm);
    border-radius: var(--sk-radius-md);
    border: 1px solid transparent;
    font: inherit;
    cursor: pointer;
  }

  .refresh-button {
    padding: var(--sk-space-xs) var(--sk-space-sm);
    background: rgba(255, 255, 255, 0.82);
    color: var(--sk-text);
    border-color: var(--sk-border-light);
  }

  .refresh-button:hover {
    border-color: rgba(0, 169, 165, 0.26);
    background: rgba(255, 255, 255, 0.96);
  }

  .refresh-intervals {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    flex-wrap: wrap;
  }

  .refresh-label {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary-strong);
  }

  .refresh-pills {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xs);
    background: rgba(255, 255, 255, 0.78);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-pill);
  }

  .refresh-pill {
    padding: var(--sk-space-xs) var(--sk-space-sm);
    background: transparent;
    color: var(--sk-secondary-strong);
  }

  .refresh-pill.is-active {
    background: rgba(0, 169, 165, 0.12);
    color: var(--sk-text);
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  .grid-refresh-toolbar.is-disabled,
  .refresh-button:disabled,
  .refresh-pill:disabled {
    opacity: 0.6;
  }

  .refresh-button:disabled,
  .refresh-pill:disabled {
    cursor: not-allowed;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }

    to {
      transform: rotate(360deg);
    }
  }
</style>
