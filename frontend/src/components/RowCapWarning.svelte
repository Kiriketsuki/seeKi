<script lang="ts">
  import type { RowCapState } from '../lib/infinite-scroll';

  let {
    capState,
    loadedCount,
  }: {
    capState: RowCapState;
    loadedCount: number;
  } = $props();
</script>

{#if capState === 'hard'}
  <div class="cap-banner cap-banner--hard" role="alert" aria-live="polite">
    <span class="cap-icon" aria-hidden="true">⚠</span>
    <span>
      Showing the first {loadedCount.toLocaleString()} rows. This table has more data — use filters
      or search to narrow your view.
    </span>
  </div>
{:else if capState === 'soft'}
  <div class="cap-banner cap-banner--soft" role="status" aria-live="polite">
    <span>
      {loadedCount.toLocaleString()} rows loaded. Scroll down to load more, or use filters to narrow
      your view.
    </span>
  </div>
{/if}

<style>
  /* ── RowCapWarning — amber warn pill/banner per app.css .sk-info-banner / .sk-status-pill.warn ── */

  /* shared structure — mirrors .sk-info-banner */
  .cap-banner {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-2xl);
    font-size: var(--sk-font-size-sm);
    font-weight: 500;
  }

  /* hard cap — danger-level amber (attention, not error) — closest to .sk-status-pill.warn */
  .cap-banner--hard {
    background: rgba(var(--sk-accent-count-rgb), 0.14);
    border-bottom: 1px solid rgba(var(--sk-accent-count-rgb), 0.28);
    color: var(--sk-accent-count-ink);
  }

  /* soft cap — gentler amber info banner — mirrors .sk-info-banner approach with amber tint */
  .cap-banner--soft {
    background: rgba(var(--sk-accent-count-rgb), 0.08);
    border-bottom: 1px solid rgba(var(--sk-accent-count-rgb), 0.18);
    color: var(--sk-accent-count-ink);
  }

  /* warning icon — flex-shrink so it never wraps */
  .cap-icon {
    flex-shrink: 0;
  }
</style>
