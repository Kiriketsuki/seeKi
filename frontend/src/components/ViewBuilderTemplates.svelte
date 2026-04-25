<script lang="ts">
  import { Layers3, CalendarRange, Trophy, Sigma, GitCompareArrows, Sparkles } from 'lucide-svelte';
  import type { TableInfo, ViewTemplateId } from '../lib/types';

  type TemplateCard = {
    id: ViewTemplateId;
    title: string;
    description: string;
    icon: typeof Layers3;
  };

  const cards: TemplateCard[] = [
    {
      id: 'most-recent-per-group',
      title: 'Most recent per group',
      description: 'Keep one row per entity and pull the newest value for each one.',
      icon: Layers3,
    },
    {
      id: 'counts-per-day',
      title: 'Counts per day',
      description: 'Bucket a timestamp by day and count how many rows landed there.',
      icon: CalendarRange,
    },
    {
      id: 'top-n-per-group',
      title: 'Top N per group',
      description: 'Rank rows inside each group and keep only the highest rows.',
      icon: Trophy,
    },
    {
      id: 'totals-by-week',
      title: 'Totals by week',
      description: 'Bucket time by week and sum a numeric measure per bucket.',
      icon: Sigma,
    },
    {
      id: 'previous-row-delta',
      title: 'Previous-row delta',
      description: 'Compare this row to the previous row for the same entity.',
      icon: GitCompareArrows,
    },
    {
      id: 'scratch',
      title: 'Start from scratch',
      description: 'Build a view column by column from the grid.',
      icon: Sparkles,
    },
  ];

  let {
    tables = [],
    baseValue = '',
    onBaseChange,
    onSelect,
    onCancel,
  }: {
    tables: TableInfo[];
    baseValue: string;
    onBaseChange: (value: string) => void;
    onSelect: (template: ViewTemplateId) => void;
    onCancel: () => void;
  } = $props();
</script>

<section class="template-gallery" data-testid="view-template-gallery">
  <div class="template-gallery__header">
    <div>
      <p class="eyebrow">Create view</p>
      <h2>Start with a ready-made shape</h2>
      <p class="copy">
        Pick a starting point, then refine it in the grid without dropping into SQL terms.
      </p>
    </div>
    <div class="template-gallery__actions">
      <button type="button" class="secondary" onclick={onCancel}>Cancel</button>
    </div>
  </div>

  <label class="field">
    <span>Starting from</span>
    <select value={baseValue} onchange={(event) => onBaseChange((event.currentTarget as HTMLSelectElement).value)}>
      {#each tables as table (`${table.schema}.${table.name}`)}
        <option value={`${table.schema}.${table.name}`}>{table.schema}.{table.name}</option>
      {/each}
    </select>
  </label>

  <div class="template-grid">
    {#each cards as card (card.id)}
      <button
        type="button"
        class="template-card"
        data-testid={`view-template-${card.id}`}
        onclick={() => onSelect(card.id)}
      >
        <span class="template-card__icon">
          <card.icon size={18} />
        </span>
        <span class="template-card__body">
          <strong>{card.title}</strong>
          <span>{card.description}</span>
        </span>
      </button>
    {/each}
  </div>
</section>

<style>
  .template-gallery {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xl);
    padding: var(--sk-space-lg) var(--sk-space-2xl);
    overflow: auto;
  }

  .template-gallery__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-lg);
  }

  .template-gallery__header h2,
  .template-gallery__header p {
    margin: 0;
  }

  .eyebrow {
    margin-bottom: var(--sk-space-xs);
    color: var(--sk-accent);
    font-size: var(--sk-font-size-xs);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .copy {
    margin-top: var(--sk-space-sm);
    color: var(--sk-secondary-strong);
    max-width: 56ch;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    max-width: 360px;
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary-strong);
  }

  .field select {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.82);
    color: var(--sk-text);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font: inherit;
  }

  .template-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--sk-space-md);
  }

  .template-card {
    display: flex;
    align-items: flex-start;
    gap: var(--sk-space-md);
    min-height: 152px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-xl);
    background:
      linear-gradient(145deg, rgba(0, 169, 165, 0.08), rgba(255, 255, 255, 0.92)),
      rgba(255, 255, 255, 0.82);
    box-shadow: var(--sk-shadow-card);
    color: var(--sk-text);
    cursor: pointer;
    padding: var(--sk-space-lg);
    text-align: left;
    transition:
      transform 0.18s ease,
      border-color 0.18s ease,
      box-shadow 0.18s ease;
  }

  .template-card:hover {
    border-color: rgba(0, 169, 165, 0.32);
    box-shadow: 0 18px 40px rgba(30, 54, 65, 0.12);
    transform: translateY(-2px);
  }

  .template-card__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.76);
    color: var(--sk-accent);
    flex-shrink: 0;
  }

  .template-card__body {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
  }

  .template-card__body strong {
    font-size: var(--sk-font-size-body);
  }

  .template-card__body span:last-child {
    color: var(--sk-secondary-strong);
    line-height: 1.45;
  }

  .secondary {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.74);
    color: var(--sk-text);
    cursor: pointer;
    font: inherit;
    padding: var(--sk-space-sm) var(--sk-space-md);
  }
</style>
