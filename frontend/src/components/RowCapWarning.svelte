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
  .cap-banner {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-xs) var(--sk-space-2xl);
    font-size: var(--sk-font-size-sm);
  }

  .cap-banner--hard {
    background: rgba(185, 28, 28, 0.07);
    border-bottom: 1px solid rgba(185, 28, 28, 0.2);
    color: #9b1c1c;
  }

  .cap-banner--soft {
    background: rgba(202, 138, 4, 0.07);
    border-bottom: 1px solid rgba(202, 138, 4, 0.18);
    color: #78350f;
  }

  .cap-icon {
    flex-shrink: 0;
  }
</style>
