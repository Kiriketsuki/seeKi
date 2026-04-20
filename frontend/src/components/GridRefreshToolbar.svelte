<script lang="ts">
  import { RefreshCw } from 'lucide-svelte';
  import { GRID_REFRESH_INTERVALS } from '../lib/constants';
  import type { GridRefreshInterval } from '../lib/stores';

  let {
    surfaceKey = '',
    intervalMs = 0,
    lastRefreshedAt = null,
    refreshing = false,
    disabled = false,
    onRefreshNow = () => {},
    onIntervalChange = () => {},
  }: {
    surfaceKey?: string;
    intervalMs?: GridRefreshInterval;
    lastRefreshedAt?: number | null;
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

  const timeFormatter = new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
  });

  let lastRefreshedLabel = $derived.by(() => {
    if (lastRefreshedAt == null) {
      return 'Not refreshed yet';
    }

    return `Last refreshed ${timeFormatter.format(new Date(lastRefreshedAt))}`;
  });
</script>

<div
  class="grid-refresh-toolbar"
  class:is-disabled={disabled}
  data-testid="grid-refresh-toolbar"
>
  <div class="grid-refresh-toolbar__main">
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

  <span class="grid-refresh-toolbar__meta" data-testid="grid-refresh-last-refreshed">
    {lastRefreshedLabel}
  </span>
</div>

<style>
  .grid-refresh-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-md);
    padding: var(--sk-space-sm) var(--sk-space-2xl) 0;
    color: var(--sk-muted);
  }

  .grid-refresh-toolbar__main {
    display: flex;
    align-items: center;
    gap: var(--sk-space-md);
    flex-wrap: wrap;
  }

  .refresh-button,
  .refresh-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: var(--sk-radius-md);
    border: 1px solid transparent;
    font: inherit;
    cursor: pointer;
  }

  .refresh-button {
    padding: 6px 10px;
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
    gap: 4px;
    padding: 4px;
    background: rgba(255, 255, 255, 0.78);
    border: 1px solid var(--sk-border-light);
    border-radius: 999px;
  }

  .refresh-pill {
    padding: 4px 10px;
    background: transparent;
    color: var(--sk-secondary-strong);
  }

  .refresh-pill.is-active {
    background: rgba(0, 169, 165, 0.12);
    color: var(--sk-text);
  }

  .grid-refresh-toolbar__meta {
    font-size: var(--sk-font-size-xs);
    white-space: nowrap;
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
