<script lang="ts">
  import {
    RevoGrid,
    type CellTemplateProp,
    type ColumnRegular,
    type ColumnTemplateProp,
    type HyperFunc,
  } from '@revolist/svelte-datagrid';
  import type { VNode } from '@revolist/revogrid';
  import type { ColumnInfo, SortDirection, SortState } from '../lib/types';
  import {
    buildSortableColumn,
    formatCellValue,
    getColumnDisplayName,
    sortStateToConfig,
  } from '../lib/data-grid';

  let {
    columns = [],
    rows = [],
    sortState,
    onSortChange,
  }: {
    columns: ColumnInfo[];
    rows: Record<string, unknown>[];
    sortState: SortState;
    onSortChange?: (column: string, direction: SortDirection | null) => void;
  } = $props();

  type SortEventDetail = {
    column: ColumnRegular;
    order?: SortDirection;
    additive: boolean;
  };

  let columnsByName = $derived(
    new Map(columns.map((column) => [column.name, column]))
  );
  let sorting = $derived(sortStateToConfig(sortState));

  function renderHeader(
    h: HyperFunc<VNode>,
    props: ColumnTemplateProp,
  ): VNode {
    const info = columnsByName.get(String(props.prop));
    const label = info ? getColumnDisplayName(info) : String(props.name ?? props.prop);
    const isSorted = sortState.column === props.prop && sortState.direction != null;

    return h(
      'div',
      {
        class: {
          'sk-grid-header': true,
          'is-sorted': isSorted,
        },
      },
      [
        h('span', { class: { 'sk-grid-header__label': true } }, label),
        h(
          'span',
          {
            class: {
              'sk-grid-header__sort': true,
              'is-active': isSorted,
            },
            'aria-hidden': 'true',
          },
          sortState.column === props.prop
            ? sortState.direction === 'asc'
              ? '▲'
              : sortState.direction === 'desc'
                ? '▼'
                : ''
            : ''
        ),
      ]
    );
  }

  function renderCell(
    h: HyperFunc<VNode>,
    props: CellTemplateProp,
  ): VNode {
    const info = columnsByName.get(String(props.prop));
    if (!info) {
      return h(
        'div',
        { class: { 'sk-grid-cell': true } },
        String(props.value ?? '')
      );
    }

    const formatted = formatCellValue(info, props.value);

    if (formatted.kind === 'null') {
      return h(
        'div',
        {
          class: {
            'sk-grid-cell': true,
            'sk-grid-cell--null': true,
          },
        },
        formatted.display
      );
    }

    if (formatted.kind === 'boolean') {
      return h(
        'div',
        { class: { 'sk-grid-cell': true, 'sk-grid-cell--boolean': true } },
        [
          h(
            'span',
            {
              class: {
                'sk-grid-badge': true,
                'is-true': formatted.booleanValue === true,
                'is-false': formatted.booleanValue === false,
              },
            },
            formatted.display
          ),
        ]
      );
    }

    return h(
      'div',
      {
        class: {
          'sk-grid-cell': true,
          'sk-grid-cell--number': formatted.kind === 'number',
          'sk-grid-cell--timestamp': formatted.kind === 'timestamp',
        },
        title: formatted.tooltip,
      },
      formatted.display
    );
  }

  function handleBeforeSorting(event: CustomEvent<SortEventDetail>) {
    event.preventDefault();
    onSortChange?.(String(event.detail.column.prop), event.detail.order ?? null);
  }

  let gridColumns: ColumnRegular[] = $derived(
    columns.map((column) =>
      buildSortableColumn(column, {
        columnTemplate: renderHeader,
        cellTemplate: renderCell,
      })
    )
  );
</script>

<div class="grid-card">
  <RevoGrid
    columns={gridColumns}
    source={rows}
    sorting={sorting}
    readonly={true}
    resize={true}
    theme="compact"
    on:beforesorting={handleBeforeSorting}
  />
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

  .grid-card :global(revo-grid) {
    --revo-grid-background: transparent;
    --revo-grid-header-bg: rgba(255, 255, 255, 0.6);
    --revo-grid-header-color: var(--sk-text);
    --revo-grid-header-border: rgba(47, 72, 88, 0.08);
    --revo-grid-cell-border: rgba(47, 72, 88, 0.06);
    --revo-grid-row-hover: rgba(0, 169, 165, 0.08);
    --revo-grid-text: var(--sk-text);
    --revo-grid-focused-bg: rgba(0, 169, 165, 0.08);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
  }

  .grid-card :global(.sk-grid-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    width: 100%;
    color: inherit;
  }

  .grid-card :global(.sk-grid-header__label) {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .grid-card :global(.sk-grid-header__sort) {
    min-width: 1ch;
    color: var(--sk-muted);
    font-size: 10px;
  }

  .grid-card :global(.sk-grid-header__sort.is-active) {
    color: var(--sk-accent);
  }

  .grid-card :global(.sk-grid-cell) {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .grid-card :global(.sk-grid-cell--number) {
    justify-content: flex-end;
    font-variant-numeric: tabular-nums;
  }

  .grid-card :global(.sk-grid-cell--timestamp) {
    color: var(--sk-secondary-strong);
    font-variant-numeric: tabular-nums;
  }

  .grid-card :global(.sk-grid-cell--boolean) {
    justify-content: center;
  }

  .grid-card :global(.sk-grid-cell--null) {
    justify-content: center;
    border-radius: var(--sk-radius-sm);
    background-image: var(--sk-null-hatch);
    color: var(--sk-muted);
    font-style: italic;
  }

  .grid-card :global(.sk-grid-badge) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 42px;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: var(--sk-font-size-sm);
    font-weight: 600;
    line-height: 1.4;
  }

  .grid-card :global(.sk-grid-badge.is-true) {
    background: rgba(22, 163, 74, 0.12);
    color: var(--sk-boolean-true);
  }

  .grid-card :global(.sk-grid-badge.is-false) {
    background: rgba(220, 38, 38, 0.12);
    color: var(--sk-boolean-false);
  }
</style>
