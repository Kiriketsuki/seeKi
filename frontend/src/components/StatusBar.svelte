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
  .statusbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-statusbar-padding-y) var(--sk-space-2xl);
    background: var(--sk-glass-statusbar);
    backdrop-filter: var(--sk-glass-statusbar-blur);
    -webkit-backdrop-filter: var(--sk-glass-statusbar-blur);
    border-top: 1px solid var(--sk-border-light);
  }

  .showing {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
  }

  .pagination {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  .page-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: var(--sk-glass-button);
    color: var(--sk-muted);
    cursor: pointer;
  }

  .page-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .page-info {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary);
    white-space: nowrap;
  }

  .page-size-control {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  .page-size-label {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    white-space: nowrap;
  }

  .page-size-select {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: var(--sk-glass-button);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-sm);
    padding: 2px var(--sk-space-sm);
    cursor: pointer;
  }

  .page-size-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
