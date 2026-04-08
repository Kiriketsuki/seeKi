<script lang="ts">
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';

  let {
    total = 0,
    start = 0,
    end = 0,
    page = 1,
    totalPages = 1,
    onPageChange,
  }: {
    total: number;
    start: number;
    end: number;
    page: number;
    totalPages: number;
    onPageChange?: (page: number) => void;
  } = $props();

  let canPrev = $derived(page > 1);
  let canNext = $derived(page < totalPages);
</script>

<div class="statusbar">
  <span class="showing">
    Showing {start.toLocaleString()} – {end.toLocaleString()} of {total.toLocaleString()}
  </span>

  <div class="pagination">
    <button
      class="page-btn"
      disabled={!canPrev}
      aria-label="Previous page"
      onclick={() => onPageChange?.(page - 1)}
    >
      <ChevronLeft size={14} />
    </button>
    <span class="page-info">{page} of {totalPages}</span>
    <button
      class="page-btn"
      disabled={!canNext}
      aria-label="Next page"
      onclick={() => onPageChange?.(page + 1)}
    >
      <ChevronRight size={14} />
    </button>
  </div>
</div>

<style>
  .statusbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-space-sm) var(--sk-space-2xl);
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
</style>
