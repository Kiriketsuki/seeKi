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

  function columnWidth(col: ColumnInfo): number {
    switch (col.data_type) {
      case 'boolean': return 80;
      case 'smallint':
      case 'integer': return 100;
      case 'bigint':
      case 'real':
      case 'double precision':
      case 'numeric': return 120;
      case 'date': return 110;
      case 'time without time zone':
      case 'time with time zone': return 100;
      case 'timestamp without time zone':
      case 'timestamp with time zone': return 180;
      case 'uuid': return 280;
      case 'json':
      case 'jsonb': return 250;
      default: return 150;
    }
  }

  let gridColumns = $derived(columns.map(col => ({
    prop: col.name,
    name: col.display_name || col.name,
    size: columnWidth(col),
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
