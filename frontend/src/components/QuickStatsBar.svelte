<script lang="ts">
  import { Sigma, TextCursorInput } from 'lucide-svelte';
  import { buildQuickStatsSnapshot } from '../lib/stores';
  import type { ColumnInfo } from '../lib/types';

  let {
    totalRows = 0,
    rows = [],
    visibleColumns = [],
    focusedTextColumnName = null,
  }: {
    totalRows?: number;
    rows?: Record<string, unknown>[];
    visibleColumns?: ColumnInfo[];
    focusedTextColumnName?: string | null;
  } = $props();

  const numberFormatter = new Intl.NumberFormat();
  const decimalFormatter = new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 2,
  });

  let snapshot = $derived.by(() =>
    buildQuickStatsSnapshot({
      totalRows,
      rows,
      visibleColumns,
      focusedTextColumnName,
    }),
  );

  function formatNumber(value: number): string {
    return Number.isInteger(value)
      ? numberFormatter.format(value)
      : decimalFormatter.format(value);
  }
</script>

<div class="quick-stats-bar" data-testid="quick-stats-bar">
  <div class="stat-chip stat-chip--primary" data-testid="quick-stat-total-rows">
    <div class="stat-chip__icon">
      <Sigma size={14} />
    </div>
    <div class="stat-chip__body">
      <span class="stat-chip__eyebrow">Filtered rows</span>
      <strong class="stat-chip__value">{numberFormatter.format(snapshot.totalRows)}</strong>
      <span class="stat-chip__meta">{numberFormatter.format(snapshot.pageRowCount)} on this page</span>
    </div>
  </div>

  {#each snapshot.numericColumns as stat (stat.columnName)}
    <div class="stat-chip" data-testid={`quick-stat-number-${stat.columnName}`}>
      <div class="stat-chip__body">
        <span class="stat-chip__eyebrow">{stat.label}</span>
        <span class="stat-chip__meta">
          Min {formatNumber(stat.min)} · Avg {formatNumber(stat.avg)} · Max {formatNumber(stat.max)}
        </span>
      </div>
    </div>
  {/each}

  {#if snapshot.focusedTextColumn}
    <div
      class="stat-chip stat-chip--text"
      data-testid={`quick-stat-text-${snapshot.focusedTextColumn.columnName}`}
    >
      <div class="stat-chip__icon">
        <TextCursorInput size={14} />
      </div>
      <div class="stat-chip__body">
        <span class="stat-chip__eyebrow">{snapshot.focusedTextColumn.label}</span>
        <strong class="stat-chip__value">
          {numberFormatter.format(snapshot.focusedTextColumn.distinctCount)} distinct
        </strong>
        <span class="stat-chip__meta">
          From {numberFormatter.format(snapshot.focusedTextColumn.sampleCount)} page values
        </span>
      </div>
    </div>
  {/if}

  <span class="quick-stats-note" data-testid="quick-stats-note">
    Page-level stats only
  </span>
</div>

<style>
  .quick-stats-bar {
    display: flex;
    align-items: stretch;
    gap: var(--sk-space-sm);
    padding: 0 var(--sk-space-2xl);
    overflow-x: auto;
  }

  .stat-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-sm);
    min-width: 0;
    padding: 10px 12px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.78);
    color: var(--sk-text);
    white-space: nowrap;
  }

  .stat-chip--primary {
    background: rgba(0, 169, 165, 0.08);
    border-color: rgba(0, 169, 165, 0.2);
  }

  .stat-chip--text {
    background: rgba(255, 149, 0, 0.08);
    border-color: rgba(255, 149, 0, 0.2);
  }

  .stat-chip__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.82);
    color: var(--sk-secondary-strong);
    flex-shrink: 0;
  }

  .stat-chip__body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .stat-chip__eyebrow {
    font-size: var(--sk-font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--sk-muted);
  }

  .stat-chip__value {
    font-size: var(--sk-font-size-body);
  }

  .stat-chip__meta {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-secondary-strong);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .quick-stats-note {
    align-self: center;
    margin-left: auto;
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
    white-space: nowrap;
  }
</style>
