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
        <span class="stat-chip__meta">
          From {numberFormatter.format(stat.sampleCount)} page values
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
  /* ── Quick Stats Bar — mirrors .sk-stats-bar from kit.css ── */
  .quick-stats-bar {
    display: flex;
    align-items: stretch;
    gap: var(--sk-space-sm);
    overflow-x: auto;
    width: 100%;
  }

  @media (max-width: 600px) {
    .quick-stats-bar {
      flex-wrap: wrap;
      overflow-x: visible;
    }
  }

  /* base chip — mirrors .sk-stat-chip */
  .stat-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-sm);
    flex: 0 0 auto;
    min-width: 0;
    padding: 6px 12px 6px 8px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-button);
    color: var(--sk-text);
    white-space: nowrap;
  }

  /* teal-tinted primary chip — mirrors .sk-stat-chip.primary */
  .stat-chip--primary {
    background: var(--sk-active-chip-bg);
    border-color: var(--sk-active-chip-border);
  }

  /* amber-tinted text chip — mirrors .sk-stat-chip.text */
  .stat-chip--text {
    background: var(--sk-count-chip-bg);
    border-color: var(--sk-count-chip-border);
  }

  /* icon container — mirrors .sk-stat-icon */
  .stat-chip__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.7);
    color: var(--sk-ink-soft);
    flex-shrink: 0;
  }

  .stat-chip--primary .stat-chip__icon {
    background: rgba(var(--sk-accent-active-rgb), 0.14);
    color: var(--sk-accent-active-strong);
  }

  .stat-chip--text .stat-chip__icon {
    background: rgba(var(--sk-accent-count-rgb), 0.16);
    color: var(--sk-accent-count-ink);
  }

  /* body stack — mirrors .sk-stat-body */
  .stat-chip__body {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }

  /* eyebrow label — mirrors .sk-stat-eyebrow */
  .stat-chip__eyebrow {
    font-size: var(--sk-font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--sk-ink-muted);
    font-weight: 600;
    line-height: 1.25;
  }

  /* primary value — mirrors .sk-stat-value */
  .stat-chip__value {
    font-size: var(--sk-font-size-md);
    font-weight: 600;
    color: var(--sk-ink-strong);
    line-height: 1.1;
    font-variant-numeric: tabular-nums;
  }

  /* secondary meta line — mirrors .sk-stat-meta */
  .stat-chip__meta {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-ink-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    font-variant-numeric: tabular-nums;
  }

  /* trailing note — mirrors .sk-stats-note */
  .quick-stats-note {
    align-self: center;
    margin-left: auto;
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
    white-space: nowrap;
    padding-left: var(--sk-space-sm);
  }
</style>
