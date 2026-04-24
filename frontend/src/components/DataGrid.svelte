<script lang="ts">
  import {
    RevoGrid,
    type CellTemplateProp,
    type ColumnRegular,
    type ColumnTemplateProp,
    type HyperFunc,
  } from '@revolist/svelte-datagrid';
  import type { VNode } from '@revolist/revogrid';
  import type {
    ColumnInfo,
    DateFormatPreference,
    FilterState,
    SortState,
  } from '../lib/types';
  import { onMount } from 'svelte';
  import {
    buildSortableColumn,
    cycleSort,
    formatCellValue,
    getColumnDisplayName,
    replaceSort,
    sortStateToConfig,
  } from '../lib/data-grid';
  import { SKELETON_ROW_MARKER } from '../lib/infinite-scroll';

  let {
    columns = [],
    rows = [],
    dateFormat = 'system',
    sortState = [],
    filters = {},
    filtersVisible = false,
    fetchingMore = false,
    resetSignal = 0,
    onSortChange,
    onFilterChange,
    onNearBottom,
    onRetryAppend,
  }: {
    columns: ColumnInfo[];
    rows: Record<string, unknown>[];
    dateFormat?: DateFormatPreference;
    sortState?: SortState;
    filters?: FilterState;
    filtersVisible?: boolean;
    fetchingMore?: boolean;
    resetSignal?: number;
    onSortChange?: (nextSortState: SortState) => void;
    onFilterChange?: (column: string, value: string) => void;
    onNearBottom?: () => void;
    onRetryAppend?: () => void;
  } = $props();

  let gridEl: HTMLDivElement | undefined = $state(undefined);

  type SortEventDetail = {
    column: ColumnRegular;
    order?: 'asc' | 'desc';
    additive: boolean;
  };

  let columnsByName = $derived(
    new Map(columns.map((column) => [column.name, column]))
  );
  let sorting = $derived(sortStateToConfig(sortState));

  function findViewportScroll(): Element | null {
    const revoEl = gridEl?.querySelector('revo-grid');
    if (!revoEl) return null;
    const shadow = revoEl.shadowRoot;
    if (!shadow) return null;
    return (
      shadow.querySelector('revogr-viewport-scroll') ??
      shadow.querySelector('[data-type="rgScrollable"]') ??
      shadow.querySelector('[class*="scroll"]') ??
      null
    );
  }

  function handleScroll(event: Event) {
    const el = event.target as Element;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (distanceFromBottom < 200) {
      onNearBottom?.();
    }
  }

  $effect(() => {
    if (!gridEl) return;

    let scrollEl: Element | null = null;

    function tryAttach(): boolean {
      scrollEl = findViewportScroll();
      if (scrollEl) {
        scrollEl.addEventListener('scroll', handleScroll, { passive: true });
        return true;
      }
      return false;
    }

    const observer = new MutationObserver(() => {
      if (tryAttach()) {
        observer.disconnect();
        return;
      }
      const revoEl = gridEl!.querySelector('revo-grid');
      if (revoEl?.shadowRoot && !observingShadow) {
        observingShadow = true;
        observer.observe(revoEl.shadowRoot, { childList: true, subtree: true });
      }
    });

    let observingShadow = false;
    if (!tryAttach()) {
      observer.observe(gridEl, { childList: true, subtree: true });
    }

    return () => {
      observer.disconnect();
      scrollEl?.removeEventListener('scroll', handleScroll);
    };
  });

  $effect(() => {
    // Track resetSignal — scroll to top whenever it changes (including on initial mount).
    // The parent only increments this counter when a genuine reset occurs.
    void resetSignal;
    const scrollEl = findViewportScroll();
    if (scrollEl) {
      (scrollEl as HTMLElement).scrollTop = 0;
    }
  });

  function renderHeader(
    h: HyperFunc<VNode>,
    props: ColumnTemplateProp,
  ): VNode {
    const info = columnsByName.get(String(props.prop));
    const label = info ? getColumnDisplayName(info) : String(props.name ?? props.prop);
    const showFilters = Boolean(
      (props as ColumnTemplateProp & { showFilters?: boolean }).showFilters
    );
    const filterValue = String(
      (props as ColumnTemplateProp & { filterValue?: string }).filterValue ?? ''
    );
    const activeSortIndex = sortState.findIndex(
      (entry) => entry.column === String(props.prop)
    );
    const activeOrder = activeSortIndex >= 0 ? sortState[activeSortIndex].direction : null;
    const isSorted = activeOrder != null;
    const sortRank = activeSortIndex >= 0 ? activeSortIndex + 1 : null;
    const sortAriaLabel =
      activeOrder != null
        ? sortState.length > 1 && sortRank != null
          ? `Sort ${activeOrder === 'asc' ? 'ascending' : 'descending'}, priority ${sortRank} of ${sortState.length}`
          : `Sort ${activeOrder === 'asc' ? 'ascending' : 'descending'}`
        : undefined;

    // `aria-sort="ascending"` on multiple columns is ambiguous; add a visually-hidden
    // text node inside the header so screen readers announce which position this column
    // holds in the sort stack (primary, secondary, etc.).
    const priorityAnnouncement =
      isSorted && sortRank != null && sortState.length > 1
        ? sortRank === 1
          ? 'Primary sort.'
          : sortRank === 2
            ? 'Secondary sort.'
            : `Sort priority ${sortRank} of ${sortState.length}.`
        : null;

    const ariaSortValue = activeOrder === 'asc'
      ? 'ascending'
      : activeOrder === 'desc'
        ? 'descending'
        : undefined;

    return h(
      'div',
      {
        class: {
          'sk-grid-header': true,
          'has-filters': showFilters,
          'is-sorted': isSorted,
        },
        ...(ariaSortValue ? { 'aria-sort': ariaSortValue } : {}),
      },
      [
        h(
          'div',
          {
            class: {
              'sk-grid-header__top': true,
            },
          },
          [
            h('span', { class: { 'sk-grid-header__label': true } }, label),
            priorityAnnouncement
              ? h('span', { class: { 'sk-sr-only': true } }, priorityAnnouncement)
              : null,
            isSorted
              ? h(
                  'span',
                  {
                    class: {
                      'sk-grid-header__sort': true,
                      'is-active': true,
                    },
                    role: 'img',
                    ...(sortAriaLabel ? { 'aria-label': sortAriaLabel, title: sortAriaLabel } : {}),
                  },
                  [
                    h('span', { 'aria-hidden': 'true' }, activeOrder === 'asc' ? '↑' : '↓'),
                    sortRank != null && sortState.length > 1
                      ? h(
                          'sup',
                          {
                            class: {
                              'sk-grid-header__sort-rank': true,
                            },
                            'aria-hidden': 'true',
                          },
                          String(sortRank)
                        )
                      : null,
                  ]
                )
              : null,
          ]
        ),
        showFilters
          ? h('div', { class: { 'sk-grid-filter': true } }, [
              h('input', {
                class: {
                  'sk-grid-filter__input': true,
                },
                type: 'text',
                value: filterValue,
                placeholder: 'Filter',
                'aria-label': `Filter ${label}`,
                onInput: (event: Event) =>
                  onFilterChange?.(
                    String(props.prop),
                    (event.target as HTMLInputElement).value
                  ),
                onClick: (event: Event) => event.stopPropagation(),
                onMouseDown: (event: Event) => event.stopPropagation(),
                onDblClick: (event: Event) => event.stopPropagation(),
                onKeyDown: (event: Event) => event.stopPropagation(),
              }),
            ])
          : null,
      ]
    );
  }

  function renderCell(
    h: HyperFunc<VNode>,
    props: CellTemplateProp,
  ): VNode {
    const model = props.model as Record<string | symbol, unknown> | undefined;
    const markerVal = model?.[SKELETON_ROW_MARKER];

    if (markerVal === 'skeleton') {
      return h('div', { class: { 'sk-skeleton-cell': true } }, [
        h('div', { class: { 'sk-skeleton-shimmer': true } }, ''),
      ]);
    }

    if (markerVal === 'error') {
      const isFirstCol = (props as CellTemplateProp & { colIndex?: number }).colIndex === 0;
      if (isFirstCol) {
        return h('div', { class: { 'sk-error-cell': true } }, [
          h('span', { class: { 'sk-error-cell__msg': true } }, 'Failed to load'),
          h(
            'button',
            {
              class: { 'sk-error-cell__retry': true },
              onclick: (e: Event) => { e.stopPropagation(); onRetryAppend?.(); },
            },
            'Retry',
          ),
        ]);
      }
      return h('div', { class: { 'sk-error-cell': true } }, '');
    }

    const info = columnsByName.get(String(props.prop));
    if (!info) {
      return h(
        'div',
        { class: { 'sk-grid-cell': true } },
        String(props.value ?? '')
      );
    }

    const formatted = formatCellValue(info, props.value, dateFormat);

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
    const column = String(event.detail.column.prop);
    const next = event.detail.additive
      ? cycleSort(sortState, column)
      : replaceSort(sortState, column);
    onSortChange?.(next);
  }

  let gridColumns: ColumnRegular[] = $derived(
    columns.map((column) =>
      buildSortableColumn(column, {
        order: sortState.find((entry) => entry.column === column.name)?.direction,
        filterValue: filters[column.name] ?? '',
        showFilters: filtersVisible,
        columnTemplate: renderHeader,
        cellTemplate: renderCell,
      })
    )
  );
</script>

<div id="data-grid" class="grid-card" class:filters-visible={filtersVisible} bind:this={gridEl}>
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
    flex-direction: column;
    align-items: stretch;
    justify-content: center;
    gap: var(--sk-grid-header-gap);
    width: 100%;
    color: inherit;
  }

  .grid-card :global(.sk-grid-header__top) {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: var(--sk-space-sm);
    width: 100%;
    min-height: 18px;
  }

  .grid-card :global(.sk-grid-header__label) {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .grid-card :global(.sk-grid-header__sort) {
    flex: 0 0 auto;
    color: var(--sk-muted);
    font-size: 10px;
  }

  .grid-card :global(.sk-grid-header__sort.is-active) {
    color: var(--sk-accent);
  }

  .grid-card :global(.sk-sr-only) {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .grid-card :global(.sk-grid-header__sort-rank) {
    font-size: 0.75em;
    line-height: 1;
    vertical-align: super;
  }

  .grid-card :global(.sk-grid-filter) {
    width: 100%;
  }

  .grid-card :global(.sk-grid-filter__input) {
    width: 100%;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: rgba(255, 255, 255, 0.8);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    line-height: 1.3;
    padding: var(--sk-grid-filter-padding-y) var(--sk-grid-filter-padding-x);
    outline: none;
  }

  .grid-card :global(.sk-grid-filter__input:focus) {
    border-color: rgba(0, 169, 165, 0.45);
    box-shadow: 0 0 0 2px rgba(0, 169, 165, 0.12);
  }

  .grid-card :global(.sk-grid-filter__input::placeholder) {
    color: var(--sk-muted);
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
    padding: var(--sk-grid-badge-padding-y) var(--sk-grid-badge-padding-x);
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

  .grid-card :global(.sk-skeleton-cell) {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    padding: 0 var(--sk-space-sm);
  }

  .grid-card :global(.sk-skeleton-shimmer) {
    width: 70%;
    height: 12px;
    border-radius: var(--sk-radius-sm);
    background: linear-gradient(
      90deg,
      rgba(47, 72, 88, 0.06) 25%,
      rgba(47, 72, 88, 0.12) 50%,
      rgba(47, 72, 88, 0.06) 75%
    );
    background-size: 200% 100%;
    animation: sk-shimmer 1.4s ease-in-out infinite;
  }

  @keyframes sk-shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  .grid-card :global(.sk-error-cell) {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    width: 100%;
    height: 100%;
    padding: 0 var(--sk-space-sm);
  }

  .grid-card :global(.sk-error-cell__msg) {
    color: #b91c1c;
    font-size: var(--sk-font-size-sm);
  }

  .grid-card :global(.sk-error-cell__retry) {
    border: 1px solid rgba(185, 28, 28, 0.3);
    border-radius: var(--sk-radius-sm);
    background: rgba(185, 28, 28, 0.06);
    color: #b91c1c;
    padding: 2px var(--sk-space-sm);
    font: inherit;
    font-size: var(--sk-font-size-sm);
    cursor: pointer;
    white-space: nowrap;
  }

  .grid-card :global(.sk-error-cell__retry:hover) {
    background: rgba(185, 28, 28, 0.12);
  }

  .filters-visible :global(revo-grid[theme='compact'] revogr-header) {
    line-height: normal;
  }

  .filters-visible :global(revo-grid[theme='compact'] revogr-header .header-rgRow) {
    height: var(--sk-grid-filter-header-height);
  }

  .filters-visible :global(revo-grid[theme='compact'] revogr-header .rgHeaderCell) {
    padding-top: 10px;
    padding-bottom: 10px;
  }
</style>
