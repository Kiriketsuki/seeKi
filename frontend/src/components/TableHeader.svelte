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
  .table-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-md);
    padding: 0;
    min-height: 52px;
    border-bottom: 1px solid var(--sk-border-light);
    background: rgba(255, 255, 255, 0.18);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
  }

  .table-info {
    display: flex;
    align-items: baseline;
    gap: var(--sk-space-sm);
    min-width: 0;
  }

  .table-name {
    font-size: var(--sk-font-size-xl);
    font-weight: 600;
    color: var(--sk-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row-count {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-secondary);
    white-space: nowrap;
  }
</style>
