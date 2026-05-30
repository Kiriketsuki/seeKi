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
    return (
      gridEl?.querySelector('revogr-scroll-virtual.vertical') ??
      gridEl?.querySelector('.vertical-inner.scroll-rgRow') ??
      gridEl?.querySelector('revogr-viewport-scroll') ??
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
      if (tryAttach()) observer.disconnect();
    });

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

    // Determine column-level alignment from data type so every cell in the column
    // (including nulls) lines up consistently regardless of the individual value.
    const colType = info.data_type;
    const isNumericCol =
      colType === 'smallint' || colType === 'integer' || colType === 'bigint' ||
      colType === 'real' || colType === 'double precision' || colType === 'numeric' ||
      colType === 'money';
    const isBooleanCol = colType === 'boolean';

    if (formatted.kind === 'null') {
      return h(
        'div',
        {
          class: {
            'sk-grid-cell': true,
            'sk-grid-cell--null': true,
            // Null cells inherit the column's alignment so the hatch pill
            // sits on the same side as the other data in the column.
            'sk-grid-cell--number': isNumericCol,
            'sk-grid-cell--boolean': isBooleanCol,
          },
        },
        [h('span', { class: { 'sk-null-pill': true } }, 'NULL')]
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
          // Use column-level isNumericCol so non-finite values (NaN/Infinity)
          // that fall through as kind:'text' still right-align with finite siblings.
          'sk-grid-cell--number': isNumericCol,
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
  /* ─── Grid card shell ──────────────────────────────────────────────────────── */
  .grid-card {
    background: var(--sk-glass-grid);
    backdrop-filter: var(--sk-glass-grid-blur);
    -webkit-backdrop-filter: var(--sk-glass-grid-blur);
    border: 1px solid var(--sk-border);
    border-radius: var(--sk-radius-lg);
    box-shadow: var(--sk-shadow-card), inset 0 1px 0 rgba(255, 255, 255, 0.55);
    overflow: hidden;
    height: 100%;
  }

  /* ─── RevoGrid CSS-variable overrides ─────────────────────────────────────── */
  .grid-card :global(revo-grid) {
    --revo-grid-background: transparent;
    /* Header: opaque frosted marble-frost slab — scrolling rows NEVER bleed through */
    --revo-grid-header-bg: transparent;
    --revo-grid-header-color: var(--sk-ink-strong);
    --revo-grid-header-border: rgba(var(--sk-ink-rgb), 0.14);
    --revo-grid-cell-border: var(--sk-border-lighter);
    --revo-grid-row-hover: rgba(var(--sk-accent-active-rgb), 0.08);
    --revo-grid-text: var(--sk-ink);
    --revo-grid-focused-bg: rgba(var(--sk-accent-active-rgb), 0.10);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    color: var(--sk-ink);
  }

  /*
   * Sticky frosted marble-frost header slab. Applied via :global so it targets
   * the shadow-DOM header element inside revo-grid. The gradient + backdrop-filter
   * ensures scrolling rows never bleed through (matches kit.css .sk-grid-table thead th).
   * A low-alpha vein wash (6–10%) is layered on top of the frost base so the header
   * re-tones across data-palette presets — deepvein reads visibly deeper/cooler while
   * alabaster stays light, without overwhelming the frosted-lift appearance.
   */
  .grid-card :global(revogr-header) {
    background:
      linear-gradient(180deg, rgba(var(--marble-vein-rgb), 0.06) 0%, rgba(var(--marble-vein-rgb), 0.10) 100%),
      linear-gradient(180deg, rgba(var(--marble-frost-rgb), 0.93) 0%, rgba(var(--marble-frost-rgb), 0.88) 100%);
    backdrop-filter: blur(18px) saturate(1.4);
    -webkit-backdrop-filter: blur(18px) saturate(1.4);
  }

  /*
   * Zebra rows — applied to the RevoGrid odd row via CSS. RevoGrid uses its own
   * row class; we override its background on even data rows to match --sk-row-alt.
   */
  .grid-card :global(revogr-data .rgRow:nth-child(even) .rgCell) {
    background: var(--sk-row-alt);
  }

  /* ─── Column header ────────────────────────────────────────────────────────── */
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
    font-weight: 600;
    color: var(--sk-ink-strong);
    letter-spacing: 0.005em;
  }

  .grid-card :global(.sk-grid-header__label) {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Sort arrow — muted when no active sort */
  .grid-card :global(.sk-grid-header__sort) {
    flex: 0 0 auto;
    color: var(--sk-ink-muted);
    font-size: var(--sk-font-size-xs);
    display: inline-flex;
    align-items: flex-start;
  }

  /* Active sort arrow — amber (count accent = selection/attention per token semantics) */
  .grid-card :global(.sk-grid-header__sort.is-active) {
    color: var(--sk-accent-count);
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
    font-size: 0.7em;
    line-height: 1;
    vertical-align: super;
  }

  /* ─── Per-column filter row ────────────────────────────────────────────────── */
  .grid-card :global(.sk-grid-filter) {
    width: 100%;
  }

  .grid-card :global(.sk-grid-filter__input) {
    width: 100%;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: var(--sk-glass-input);
    color: var(--sk-ink);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    line-height: 1.3;
    padding: var(--sk-grid-filter-padding-y) var(--sk-grid-filter-padding-x);
    outline: none;
  }

  .grid-card :global(.sk-grid-filter__input:focus) {
    border-color: var(--sk-ring-border);
    box-shadow: 0 0 0 2px var(--sk-ring-data);
  }

  .grid-card :global(.sk-grid-filter__input::placeholder) {
    color: var(--sk-ink-muted);
  }

  /* ─── Data cells ───────────────────────────────────────────────────────────── */
  .grid-card :global(.sk-grid-cell) {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--sk-ink);
    /* Default: left-aligned text */
    justify-content: flex-start;
  }

  /* Numeric columns: right-aligned, tabular figures */
  .grid-card :global(.sk-grid-cell--number) {
    justify-content: flex-end;
    /* text-align mirrors justify-content so getComputedStyle reports 'right' (e2e: data-grid.spec.ts:448) */
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-feature-settings: 'tnum' 1;
  }

  /* Timestamp columns: secondary color, tabular figures, left-aligned */
  .grid-card :global(.sk-grid-cell--timestamp) {
    color: var(--sk-ink-soft);
    font-variant-numeric: tabular-nums;
    font-feature-settings: 'tnum' 1;
  }

  /* Boolean columns: center-aligned */
  .grid-card :global(.sk-grid-cell--boolean) {
    justify-content: center;
  }

  /*
   * NULL cells: hatched pill. Alignment is inherited from sibling column-type classes
   * (e.g. sk-grid-cell--number) — a null in a numeric column still right-aligns.
   * The display rule below keeps the block non-empty so linters don't flag it.
   */
  .grid-card :global(.sk-grid-cell--null) {
    display: flex; /* inherited from .sk-grid-cell; explicit here to satisfy lint */
  }

  .grid-card :global(.sk-null-pill) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 10px;
    border-radius: var(--sk-radius-sm);
    background-image: var(--sk-null-hatch);
    color: var(--sk-ink-muted);
    font-style: italic;
    font-size: var(--sk-font-size-sm);
  }

  /* ─── Boolean badges ───────────────────────────────────────────────────────── */
  .grid-card :global(.sk-grid-badge) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 50px;
    padding: var(--sk-grid-badge-padding-y) var(--sk-grid-badge-padding-x);
    border-radius: var(--sk-radius-pill);
    font-size: var(--sk-font-size-sm);
    font-weight: 600;
    line-height: 1.4;
  }

  .grid-card :global(.sk-grid-badge.is-true) {
    background: rgba(var(--sk-boolean-true-rgb), 0.12);
    color: var(--sk-boolean-true);
  }

  .grid-card :global(.sk-grid-badge.is-false) {
    background: rgba(var(--sk-boolean-false-rgb), 0.12);
    color: var(--sk-boolean-false);
  }

  /* ─── Skeleton loading cells ───────────────────────────────────────────────── */
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
      rgba(var(--sk-ink-rgb), 0.06) 25%,
      rgba(var(--sk-ink-rgb), 0.12) 50%,
      rgba(var(--sk-ink-rgb), 0.06) 75%
    );
    background-size: 200% 100%;
    animation: sk-shimmer 1.4s ease-in-out infinite;
  }

  @keyframes sk-shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  /* ─── Error / retry cells ──────────────────────────────────────────────────── */
  .grid-card :global(.sk-error-cell) {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    width: 100%;
    height: 100%;
    padding: 0 var(--sk-space-sm);
  }

  .grid-card :global(.sk-error-cell__msg) {
    color: var(--sk-danger);
    font-size: var(--sk-font-size-sm);
  }

  .grid-card :global(.sk-error-cell__retry) {
    border: 1px solid rgba(var(--sk-danger-rgb), 0.3);
    border-radius: var(--sk-radius-sm);
    background: rgba(var(--sk-danger-rgb), 0.06);
    color: var(--sk-danger);
    padding: 2px var(--sk-space-sm);
    font: inherit;
    font-size: var(--sk-font-size-sm);
    cursor: pointer;
    white-space: nowrap;
  }

  .grid-card :global(.sk-error-cell__retry:hover) {
    background: rgba(var(--sk-danger-rgb), 0.12);
  }

  /* ─── Filter-visible header height fix ────────────────────────────────────── */
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
