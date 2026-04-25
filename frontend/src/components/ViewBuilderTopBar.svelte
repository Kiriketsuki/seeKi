<script lang="ts">
  import { Filter, Layers3, Save, X } from 'lucide-svelte';
  import type { TableInfo } from '../lib/types';

  export type BuilderOption = {
    value: string;
    label: string;
  };

  let {
    name = '',
    baseValue = '',
    tables = [],
    sourceLabel = '',
    templateLabel = '',
    groupingKey = '',
    groupingOptions = [],
    latestBy = '',
    latestOptions = [],
    rankBy = '',
    rankingEnabled = false,
    rankLimit = 3,
    filterCount = 0,
    saving = false,
    onNameChange,
    onBaseChange,
    onGroupingChange,
    onLatestByChange,
    onRankByChange,
    onToggleRanking,
    onRankLimitChange,
    onToggleFilters,
    onSave,
    onCancel,
  }: {
    name: string;
    baseValue: string;
    tables: TableInfo[];
    sourceLabel?: string;
    templateLabel?: string;
    groupingKey: string;
    groupingOptions: BuilderOption[];
    latestBy: string;
    latestOptions: BuilderOption[];
    rankBy: string;
    rankingEnabled: boolean;
    rankLimit: number;
    filterCount: number;
    saving?: boolean;
    onNameChange: (value: string) => void;
    onBaseChange: (value: string) => void;
    onGroupingChange: (value: string) => void;
    onLatestByChange: (value: string) => void;
    onRankByChange: (value: string) => void;
    onToggleRanking: () => void;
    onRankLimitChange: (value: number) => void;
    onToggleFilters: () => void;
    onSave: () => void;
    onCancel: () => void;
  } = $props();

  const topNLabel = $derived.by(() => (rankingEnabled ? `Top ${rankLimit}` : 'Top N per group'));
</script>

<section class="builder-topbar" data-testid="view-builder-topbar">
  <div class="builder-topbar__cluster builder-topbar__cluster--wide">
    <label class="field field--name">
      <span>Name</span>
      <input
        value={name}
        type="text"
        placeholder="Vehicle battery health"
        data-testid="view-builder-name"
        oninput={(event) => onNameChange((event.currentTarget as HTMLInputElement).value)}
      />
    </label>

    <div class="builder-pills">
      {#if templateLabel}
        <span class="pill pill--accent">
          <Layers3 size={14} />
          <span>{templateLabel}</span>
        </span>
      {/if}
      {#if sourceLabel}
        <span class="pill">Started from {sourceLabel}</span>
      {/if}
    </div>
  </div>

  <div class="builder-topbar__cluster">
    <label class="field">
      <span>Starting from</span>
      <select value={baseValue} data-testid="view-builder-base-table" onchange={(event) => onBaseChange((event.currentTarget as HTMLSelectElement).value)}>
        {#each tables as table (`${table.schema}.${table.name}`)}
          <option value={`${table.schema}.${table.name}`}>{table.schema}.{table.name}</option>
        {/each}
      </select>
    </label>

    <label class="field">
      <span>One row per</span>
      <select value={groupingKey} data-testid="view-builder-grouping" onchange={(event) => onGroupingChange((event.currentTarget as HTMLSelectElement).value)}>
        <option value="">Keep all rows</option>
        {#each groupingOptions as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    </label>

    <label class="field">
      <span>Latest by</span>
      <select value={latestBy} data-testid="view-builder-latest-by" onchange={(event) => onLatestByChange((event.currentTarget as HTMLSelectElement).value)}>
        <option value="">Not set</option>
        {#each latestOptions as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    </label>

    <div class="field field--top-n">
      <span>{topNLabel}</span>
      <div class="top-n-controls">
        <button
          type="button"
          class="toggle-chip"
          class:active={rankingEnabled}
          data-testid="view-builder-ranking-toggle"
          onclick={onToggleRanking}
        >
          Top N per group
        </button>
        {#if rankingEnabled}
          <input
            type="number"
            min="1"
            max="25"
            value={String(rankLimit)}
            data-testid="view-builder-ranking-limit"
            oninput={(event) => onRankLimitChange(Number((event.currentTarget as HTMLInputElement).value))}
          />
          <select value={rankBy} data-testid="view-builder-rank-by" onchange={(event) => onRankByChange((event.currentTarget as HTMLSelectElement).value)}>
            <option value="">Rank by…</option>
            {#each latestOptions as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        {/if}
      </div>
    </div>
  </div>

  <div class="builder-topbar__cluster builder-topbar__cluster--actions">
    <button type="button" class="secondary" data-testid="view-builder-filters" onclick={onToggleFilters}>
      <Filter size={14} />
      <span>Filters{filterCount > 0 ? ` (${filterCount})` : ''}</span>
    </button>
    <button type="button" class="secondary" onclick={onCancel}>
      <X size={14} />
      <span>Cancel</span>
    </button>
    <button type="button" class="primary" data-testid="view-builder-save" disabled={saving} onclick={onSave}>
      <Save size={14} />
      <span>{saving ? 'Saving…' : 'Save'}</span>
    </button>
  </div>
</section>

<style>
  .builder-topbar {
    display: grid;
    grid-template-columns: minmax(220px, 2fr) repeat(2, minmax(0, 1fr));
    gap: var(--sk-space-lg);
    align-items: start;
    padding: var(--sk-space-lg);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-xl);
    background:
      radial-gradient(circle at top left, rgba(0, 169, 165, 0.08), transparent 34%),
      rgba(255, 255, 255, 0.82);
    box-shadow: var(--sk-shadow-card);
  }

  .builder-topbar__cluster {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-md);
    min-width: 0;
  }

  .builder-topbar__cluster--wide {
    flex-direction: column;
  }

  .builder-topbar__cluster--actions {
    justify-content: flex-end;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    min-width: 0;
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary-strong);
  }

  .field--name {
    min-width: 260px;
  }

  .field span {
    white-space: nowrap;
  }

  .field input,
  .field select,
  .top-n-controls input {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.84);
    color: var(--sk-text);
    font: inherit;
    padding: var(--sk-space-sm) var(--sk-space-sm);
  }

  .builder-pills {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-sm);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-sm);
    border-radius: var(--sk-radius-pill);
    background: rgba(47, 72, 88, 0.08);
    color: var(--sk-secondary-strong);
    padding: var(--sk-space-sm) var(--sk-space-sm);
    font-size: var(--sk-font-size-xs);
    font-weight: 600;
  }

  .pill--accent {
    background: rgba(0, 169, 165, 0.1);
    color: var(--sk-accent);
  }

  .field--top-n {
    min-width: 240px;
  }

  .top-n-controls {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  .top-n-controls input {
    width: 80px;
  }

  .toggle-chip,
  .secondary,
  .primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-sm);
    border-radius: var(--sk-radius-md);
    cursor: pointer;
    font: inherit;
    padding: var(--sk-space-sm) var(--sk-space-md);
  }

  .toggle-chip,
  .secondary {
    border: 1px solid var(--sk-border-light);
    background: rgba(255, 255, 255, 0.72);
    color: var(--sk-text);
  }

  .toggle-chip.active {
    border-color: rgba(0, 169, 165, 0.32);
    background: rgba(0, 169, 165, 0.08);
    color: var(--sk-accent);
  }

  .primary {
    border: 1px solid rgba(0, 169, 165, 0.28);
    background: rgba(0, 169, 165, 0.12);
    color: var(--sk-accent);
  }

  .primary:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  @media (max-width: 1280px) {
    .builder-topbar {
      grid-template-columns: minmax(0, 1fr);
    }

    .builder-topbar__cluster--actions {
      justify-content: flex-start;
    }
  }
</style>
