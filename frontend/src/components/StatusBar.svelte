<script lang="ts">
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import type { PageSizePreference, PaginationMode } from '../lib/types';

  let {
    mode = 'paged',
    total = 0,
    loadedCount = 0,
    start = 0,
    end = 0,
    page = 1,
    totalPages = 1,
    pageSize = 50,
    loading = false,
    onPageChange,
    onPageSizeChange,
  }: {
    mode?: PaginationMode;
    total: number;
    loadedCount?: number;
    start: number;
    end: number;
    page: number;
    totalPages: number;
    pageSize?: PageSizePreference;
    loading?: boolean;
    onPageChange?: (page: number) => void;
    onPageSizeChange?: (size: PageSizePreference) => void;
  } = $props();

  const PAGE_SIZES: PageSizePreference[] = [50, 100, 250, 500];

  let canPrev = $derived(page > 1 && !loading);
  let canNext = $derived(page < totalPages && !loading);
</script>

<div class="statusbar">
  {#if mode === 'infinite'}
    <span class="showing">
      Loaded {loadedCount.toLocaleString()} of {total.toLocaleString()}
    </span>
    <div class="page-size-control">
      <label class="page-size-label" for="sk-page-size">Rows per fetch</label>
      <select
        id="sk-page-size"
        class="page-size-select"
        value={pageSize}
        disabled={loading}
        onchange={(e) => {
          const val = Number((e.currentTarget as HTMLSelectElement).value) as PageSizePreference;
          onPageSizeChange?.(val);
        }}
      >
        {#each PAGE_SIZES as size}
          <option value={size}>{size}</option>
        {/each}
      </select>
    </div>
  {:else}
    <span class="showing">
      Showing {start.toLocaleString()} - {end.toLocaleString()} of {total.toLocaleString()}
    </span>

    <div class="pagination">
      <button
        type="button"
        class="page-btn"
        disabled={!canPrev}
        aria-label="Previous page"
        onclick={() => onPageChange?.(page - 1)}
      >
        <ChevronLeft size={14} />
      </button>
      <span class="page-info">{page} of {totalPages}</span>
      <button
        type="button"
        class="page-btn"
        disabled={!canNext}
        aria-label="Next page"
        onclick={() => onPageChange?.(page + 1)}
      >
        <ChevronRight size={14} />
      </button>
    </div>
  {/if}
</div>

<style>
  /* ── StatusBar — mirrors .sk-statusbar from app/app.css ── */
  .statusbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-statusbar-padding-y) var(--sk-space-2xl);
    background: var(--sk-glass-statusbar);
    backdrop-filter: var(--sk-glass-statusbar-blur);
    -webkit-backdrop-filter: var(--sk-glass-statusbar-blur);
    border-top: 1px solid var(--sk-border-light);
    flex-shrink: 0;
  }

  /* "Showing X–Y of Z" / "Loaded N of M" — mirrors .sk-sb-showing */
  .showing {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    font-variant-numeric: tabular-nums;
  }

  /* pagination cluster — mirrors .sk-sb-pagination */
  .pagination {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  /* prev/next buttons — mirrors .sk-sb-page-btn (teal hover) */
  .page-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: var(--sk-glass-button);
    color: var(--sk-muted);
    cursor: pointer;
    transition: color 0.12s ease, border-color 0.12s ease;
  }

  .page-btn:hover:not(:disabled) {
    color: var(--sk-text);
    border-color: rgba(var(--sk-accent-active-rgb), 0.24);
  }

  .page-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  /* "X of Y" — mirrors .sk-sb-page-info */
  .page-info {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary-strong);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  /* rows-per-fetch cluster — mirrors .sk-sb-pagesize */
  .page-size-control {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  /* label — mirrors .sk-sb-pagesize-label */
  .page-size-label {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    white-space: nowrap;
  }

  /* select — mirrors .sk-sb-select (app.css: .sk-select/.sk-sb-select rules) */
  .page-size-select {
    appearance: none;
    -webkit-appearance: none;
    border: 1px solid var(--sk-border-input);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-sm);
    padding: 3px calc(var(--sk-space-lg) + 4px) 3px var(--sk-space-sm);
    cursor: pointer;
    outline: none;
    transition: border-color 0.12s ease, box-shadow 0.12s ease;
  }

  .page-size-select:focus {
    border-color: rgba(var(--sk-accent-active-rgb), 0.45);
    box-shadow: 0 0 0 2px var(--sk-ring-data);
  }

  .page-size-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
