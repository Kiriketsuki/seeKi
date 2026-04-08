<script lang="ts">
  import { RevoGrid } from '@revolist/svelte-datagrid';
  import type { ColumnInfo } from '../lib/types';

  let {
    columns = [],
    rows = [],
  }: {
    columns: ColumnInfo[];
    rows: Record<string, unknown>[];
  } = $props();

  let gridColumns = $derived(columns.map(col => ({
    prop: col.name,
    name: col.display_name || col.name,
    size: 150,
  })));
</script>

<div class="grid-card">
  <RevoGrid columns={gridColumns} source={rows} theme="compact" />
</div>

<style>
  .grid-card {
    background: var(--sk-glass-grid);
    backdrop-filter: var(--sk-glass-grid-blur);
    -webkit-backdrop-filter: var(--sk-glass-grid-blur);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    box-shadow: var(--sk-shadow-card);
    overflow: hidden;
    height: 100%;
  }
</style>
