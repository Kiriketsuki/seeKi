<script lang="ts">
  let {
    tableName = '',
    rowCount = 0,
  }: {
    tableName: string;
    rowCount: number;
  } = $props();
</script>

<header class="table-header">
  <div class="table-info">
    <span class="table-name">{tableName}</span>
    <span class="row-count">{rowCount.toLocaleString()} rows</span>
  </div>
</header>

<style>
  /* Matches .sk-table-panel + .sk-table-header from kit.css */
  .table-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-md);
    /* No background — the panel padding wraps this; let App.svelte / parent handle bg */
    padding: 0;
    min-height: var(--sk-table-header-min-height);
  }

  /* Table name + row count sit on the baseline together */
  .table-info {
    display: flex;
    align-items: baseline;
    gap: var(--sk-space-md);
    min-width: 0;
  }

  /* Matches .sk-table-name: xl/600/tight tracking */
  .table-name {
    font-size: var(--sk-font-size-xl);
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--sk-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Matches .sk-table-rowcount: body size, muted, tabular figures */
  .row-count {
    font-size: var(--sk-font-size-body);
    color: var(--sk-ink-muted);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
    font-feature-settings: 'tnum' 1;
  }
</style>
