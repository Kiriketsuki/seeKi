<script lang="ts">
  import { createView, fetchColumns, fetchFkPath, previewView } from '../lib/api';
  import type {
    ColumnInfo,
    QueryResult,
    SavedViewSummary,
    TableInfo,
    ViewColumn,
    ViewColumnRef,
    ViewDefinitionFilters,
    ViewDraft,
    ViewFilterValue,
    ViewGrouping,
    ViewOrderBy,
    ViewRanking,
    ViewSourceRef,
    ViewTemplateId,
  } from '../lib/types';
  import ColumnPicker from './ColumnPicker.svelte';
  import ViewBuilderGrid from './ViewBuilderGrid.svelte';
  import ViewBuilderTemplates from './ViewBuilderTemplates.svelte';
  import ViewBuilderTopBar, { type BuilderOption } from './ViewBuilderTopBar.svelte';

  type PickerPayload = {
    column: ViewColumn;
    source: ViewSourceRef | null;
  };

  type FilterRow = {
    key: string;
    label: string;
    filter: ViewFilterValue | null;
  };

  type BuilderGridItem = {
    id: string;
    label: string;
    detail: string;
    kind: 'source' | 'derived';
  };

  let {
    tables = [],
    initialDraft = null,
    sourceLabel = '',
    onCancel,
    onSaved,
    onDraftChange,
  }: {
    tables: TableInfo[];
    initialDraft: ViewDraft | null;
    sourceLabel?: string;
    onCancel: () => void;
    onSaved: (view: SavedViewSummary) => Promise<void> | void;
    onDraftChange?: (draft: { columns: ViewColumn[]; sources: ViewSourceRef[]; grouping: ViewGrouping | null; ranking: ViewRanking | null }) => void;
  } = $props();

  const templateLabels: Record<ViewTemplateId, string> = {
    scratch: 'Start from scratch',
    'most-recent-per-group': 'Most recent per group',
    'counts-per-day': 'Counts per day',
    'top-n-per-group': 'Top N per group',
    'totals-by-week': 'Totals by week',
    'previous-row-delta': 'Previous-row delta',
  };

  let name = $state('');
  let baseSchema = $state('');
  let baseTable = $state('');
  let columns = $state<ViewColumn[]>([]);
  let filters = $state<ViewDefinitionFilters>({});
  let sources = $state<ViewSourceRef[]>([]);
  let grouping = $state<ViewGrouping | null>(null);
  let ranking = $state<ViewRanking | null>(null);
  let template = $state<ViewTemplateId | null>(null);
  let preview = $state<QueryResult | null>(null);
  let previewLoading = $state(false);
  let reachableTables = $state<TableInfo[]>([]);
  let pickerOpen = $state(false);
  let pickerIndex = $state<number | null>(null);
  let pickerValue = $state<ViewColumn | null>(null);
  let error = $state('');
  let saving = $state(false);
  let showFilters = $state(false);
  let previewTimer: ReturnType<typeof setTimeout> | null = null;
  let previewRequestId = 0;
  let reachabilityRequestId = 0;
  let seedKey = $state('');
  let loadedColumns = $state<Record<string, ColumnInfo[]>>({});
  let templateStage = $state(true);
  let upgradeConfirmOpen = $state(false);

  // onDraftChange is a reactive dep here (Svelte 5 $props() tracking).
  // builderDraftLive must NOT appear in App.svelte's reactive template — only in handlers —
  // otherwise the cascade de5fd53 prevents would silently reintroduce itself.
  $effect(() => {
    onDraftChange?.({ columns, sources, grouping, ranking });
  });

  function tableKey(schema: string, table: string): string {
    return `${schema}.${table}`;
  }

  function encodeColumnRef(ref: ViewColumnRef | ViewOrderBy | null | undefined): string {
    if (!ref) return '';
    return [ref.source_id ?? '', ref.source_schema, ref.source_table, ref.column_name].join('|');
  }

  function decodeColumnRef(value: string): ViewColumnRef | null {
    if (!value) return null;
    const [sourceId, sourceSchema, sourceTable, columnName] = value.split('|');
    if (!sourceSchema || !sourceTable || !columnName) return null;
    return {
      source_id: sourceId || null,
      source_schema: sourceSchema,
      source_table: sourceTable,
      column_name: columnName,
    };
  }

  function toOrderBy(value: string, direction: 'asc' | 'desc'): ViewOrderBy | null {
    const ref = decodeColumnRef(value);
    if (!ref) return null;
    return {
      ...ref,
      direction,
    };
  }

  function isNumeric(dataType: string): boolean {
    return ['smallint', 'integer', 'bigint', 'numeric', 'real', 'double precision'].includes(dataType);
  }

  function isTemporal(dataType: string): boolean {
    return ['date', 'timestamp', 'timestamp without time zone', 'timestamp with time zone', 'timestamptz'].includes(dataType);
  }

  function prettyFieldLabel(ref: ViewColumnRef, info: ColumnInfo | null): string {
    const sourcePrefix = ref.source_id ? `${ref.source_id} → ` : `${ref.source_table} → `;
    return `${sourcePrefix}${info?.display_name ?? ref.column_name}`;
  }

  function cloneColumn(column: ViewColumn): ViewColumn {
    return {
      ...column,
      derived: column.derived
        ? {
            ...column.derived,
            inputs: column.derived.inputs.map((input) => ({ ...input })),
            options: column.derived.options ? { ...column.derived.options } : column.derived.options,
          }
        : column.derived,
    };
  }

  function cloneDraft(source: ViewDraft | null): ViewDraft {
    if (!source) {
      const firstTable = tables[0];
      return {
        name: '',
        base_schema: firstTable?.schema ?? '',
        base_table: firstTable?.name ?? '',
        definition_version: 2,
        columns: [],
        filters: {},
        sources: [],
        grouping: null,
        ranking: null,
        template: null,
      };
    }

    return {
      name: source.name,
      base_schema: source.base_schema,
      base_table: source.base_table,
      definition_version: source.definition_version,
      columns: source.columns.map(cloneColumn),
      filters: structuredClone(source.filters ?? {}),
      sources: source.sources?.map((item) => structuredClone(item)) ?? [],
      grouping: source.grouping ? structuredClone(source.grouping) : null,
      ranking: source.ranking ? structuredClone(source.ranking) : null,
      template: source.template ?? null,
    };
  }

  function resetToDraft(nextDraft: ViewDraft | null) {
    const draft = cloneDraft(nextDraft);
    name = draft.name;
    baseSchema = draft.base_schema;
    baseTable = draft.base_table;
    columns = draft.columns;
    filters = draft.filters;
    sources = draft.sources ?? [];
    grouping = draft.grouping ?? null;
    ranking = draft.ranking ?? null;
    template = draft.template ?? null;
    preview = null;
    error = '';
    showFilters = false;
    pickerOpen = false;
    pickerIndex = null;
    pickerValue = null;
    templateStage = draft.columns.length === 0 && !draft.template;
  }

  $effect(() => {
    const nextSeedKey = JSON.stringify(initialDraft ?? { empty: true });
    if (seedKey === nextSeedKey) return;
    seedKey = nextSeedKey;
    resetToDraft(initialDraft);
  });

  async function ensureColumns(schema: string, table: string) {
    const key = tableKey(schema, table);
    if (loadedColumns[key]) return loadedColumns[key];
    const nextColumns = await fetchColumns(schema, table);
    loadedColumns = {
      ...loadedColumns,
      [key]: nextColumns,
    };
    return nextColumns;
  }

  $effect(() => {
    if (!baseSchema || !baseTable) return;
    void ensureColumns(baseSchema, baseTable);
  });

  $effect(() => {
    for (const source of sources) {
      void ensureColumns(source.schema, source.table);
    }
  });

  async function computeReachableTables(schema: string, table: string) {
    if (!schema || !table) {
      reachableTables = [];
      return;
    }
    const myRequest = ++reachabilityRequestId;
    try {
      const sameSchemaTables = tables.filter((candidate) => candidate.schema === schema);
      const reachable = await Promise.all(
        sameSchemaTables.map(async (candidate) => {
          if (candidate.schema === schema && candidate.name === table) return true;
          try {
            const path = await fetchFkPath(schema, table, candidate.schema, candidate.name);
            return path.length > 0;
          } catch {
            return false;
          }
        }),
      );
      if (myRequest !== reachabilityRequestId) return;
      reachableTables = sameSchemaTables.filter((_, index) => reachable[index]);
    } finally {
      if (myRequest === reachabilityRequestId) {
        // no-op
      }
    }
  }

  let reachabilityTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (!baseSchema || !baseTable) return;
    const schema = baseSchema;
    const table = baseTable;
    clearTimeout(reachabilityTimer);
    reachabilityTimer = setTimeout(() => void computeReachableTables(schema, table), 300);
    return () => clearTimeout(reachabilityTimer);
  });

  function outputNameForColumn(column: ViewColumn): string {
    if (column.derived?.alias?.trim()) return column.derived.alias.trim();
    if (column.alias?.trim()) return column.alias.trim();
    if (column.aggregate) {
      const prefix = column.source_id ?? column.source_table;
      return `${column.aggregate.toLowerCase()}_${prefix}__${column.column_name}`;
    }
    if (column.kind === 'derived') {
      return `${column.column_name}_derived`;
    }
    const baseName = column.column_name;
    const duplicates = columns.filter((candidate) => candidate.column_name === baseName).length;
    if (duplicates > 1) {
      return `${column.source_id ?? column.source_table}__${baseName}`;
    }
    return baseName;
  }

  const filterRows = $derived.by<FilterRow[]>(() =>
    columns
      .filter((column) => column.aggregate == null)
      .map((column) => {
        const key = outputNameForColumn(column);
        return {
          key,
          label: key,
          filter: filters[key] ?? null,
        };
      }),
  );

  const baseColumns = $derived.by(() => loadedColumns[tableKey(baseSchema, baseTable)] ?? []);

  const fieldOptions = $derived.by(() => {
    const refs: BuilderOption[] = [];
    const refColumns = [{ id: null, schema: baseSchema, table: baseTable }, ...sources.map((source) => ({
      id: source.id,
      schema: source.schema,
      table: source.table,
    }))];

    for (const ref of refColumns) {
      const infoList = loadedColumns[tableKey(ref.schema, ref.table)] ?? [];
      for (const info of infoList) {
        const value = encodeColumnRef({
          source_id: ref.id,
          source_schema: ref.schema,
          source_table: ref.table,
          column_name: info.name,
        });
        refs.push({
          value,
          label: prettyFieldLabel(
            {
              source_id: ref.id,
              source_schema: ref.schema,
              source_table: ref.table,
              column_name: info.name,
            },
            info,
          ),
        });
      }
    }

    return refs;
  });

  const latestOptions = $derived.by(() =>
    fieldOptions.filter((option) => {
      const ref = decodeColumnRef(option.value);
      if (!ref) return false;
      const info = (loadedColumns[tableKey(ref.source_schema, ref.source_table)] ?? []).find(
        (candidate) => candidate.name === ref.column_name,
      );
      return info ? isTemporal(info.data_type) || isNumeric(info.data_type) : false;
    }),
  );

  const currentGroupingValue = $derived.by(() => encodeColumnRef(grouping?.keys[0]));
  const currentLatestValue = $derived.by(() => encodeColumnRef(grouping?.latest_by));
  const currentRankValue = $derived.by(() => encodeColumnRef(ranking?.order_by));
  const currentFilterCount = $derived.by(() => {
    const activeKeys = new Set(filterRows.map((row) => row.key));
    return Object.keys(filters).filter((key) => activeKeys.has(key) && filters[key] != null).length;
  });
  const builderItems = $derived.by<BuilderGridItem[]>(() =>
    columns.map((column, index) => ({
      id: `${outputNameForColumn(column)}-${index}`,
      label: outputNameForColumn(column),
      detail: column.derived
        ? column.derived.operation.replaceAll('_', ' ')
        : `${column.source_id ?? column.source_table}.${column.column_name}${column.aggregate ? ` • ${column.aggregate}` : ''}`,
      kind: column.kind === 'derived' ? 'derived' : 'source',
    })),
  );

  const templateLabel = $derived.by(() => (template ? templateLabels[template] : ''));

  function buildDraft(): ViewDraft {
    return {
      name,
      base_schema: baseSchema,
      base_table: baseTable,
      definition_version: initialDraft?.definition_version ?? 2,
      columns,
      filters,
      sources,
      grouping,
      ranking,
      template,
    };
  }

  $effect(() => {
    if (previewTimer) clearTimeout(previewTimer);
    if (!baseSchema || !baseTable || columns.length === 0 || templateStage) {
      preview = null;
      previewLoading = false;
      return;
    }

    const myRequest = ++previewRequestId;
    previewLoading = true;
    previewTimer = setTimeout(async () => {
      try {
        const result = await previewView(buildDraft());
        if (myRequest !== previewRequestId) return;
        preview = result;
        error = '';
      } catch (err) {
        if (myRequest !== previewRequestId) return;
        error = err instanceof Error ? err.message : 'Preview failed';
        preview = null;
      } finally {
        if (myRequest === previewRequestId) {
          previewLoading = false;
        }
      }
    }, 300);

    return () => {
      if (previewTimer) clearTimeout(previewTimer);
    };
  });

  function chooseLikelyColumn(
    list: ColumnInfo[],
    matcher: (column: ColumnInfo) => boolean,
    fallback?: (column: ColumnInfo) => boolean,
  ): ColumnInfo | null {
    return list.find(matcher) ?? (fallback ? list.find(fallback) ?? null : list[0] ?? null);
  }

  function chooseEntityColumn(list: ColumnInfo[]): ColumnInfo | null {
    return (
      list.find((column) => column.name.endsWith('_id') && !column.is_primary_key) ??
      list.find(
        (column) =>
          !column.is_primary_key &&
          !isTemporal(column.data_type) &&
          !isNumeric(column.data_type),
      ) ??
      list.find((column) => column.name === 'id' && !column.is_primary_key) ??
      list.find((column) => !column.is_primary_key && !isTemporal(column.data_type)) ??
      list[0] ??
      null
    );
  }

  function chooseTimestampColumn(list: ColumnInfo[]): ColumnInfo | null {
    return chooseLikelyColumn(
      list,
      (column) => isTemporal(column.data_type) && (column.name.includes('time') || column.name.includes('at')),
      (column) => isTemporal(column.data_type),
    );
  }

  function chooseNumericColumn(list: ColumnInfo[]): ColumnInfo | null {
    return chooseLikelyColumn(
      list,
      (column) =>
        isNumeric(column.data_type) &&
        !column.is_primary_key &&
        column.name !== 'id' &&
        !column.name.endsWith('_id'),
      (column) =>
        isNumeric(column.data_type) &&
        !column.is_primary_key &&
        column.name !== 'id',
    );
  }

  function chooseLabelColumn(list: ColumnInfo[]): ColumnInfo | null {
    return chooseLikelyColumn(
      list,
      (column) => ['label', 'name', 'title', 'event_type'].includes(column.name),
      (column) => !isNumeric(column.data_type) && !isTemporal(column.data_type),
    );
  }

  async function seedTemplate(nextTemplate: ViewTemplateId) {
    const currentColumns = await ensureColumns(baseSchema, baseTable);
    const entityColumn = chooseEntityColumn(currentColumns);
    const timestampColumn = chooseTimestampColumn(currentColumns);
    const numericColumn = chooseNumericColumn(currentColumns);
    const labelColumn = chooseLabelColumn(currentColumns);

    template = nextTemplate;
    filters = {};
    sources = [];
    grouping = null;
    ranking = null;
    columns = [];
    error = '';

    if (nextTemplate === 'scratch') {
      templateStage = false;
      return;
    }

    if (nextTemplate === 'most-recent-per-group' && entityColumn && timestampColumn) {
      columns = [
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: entityColumn.name,
          alias: entityColumn.name,
          aggregate: null,
        },
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: (numericColumn ?? labelColumn ?? timestampColumn).name,
          alias: numericColumn?.name ?? labelColumn?.name ?? timestampColumn.name,
          aggregate: 'LATEST',
        },
      ];
      grouping = {
        keys: [
          {
            source_id: null,
            source_schema: baseSchema,
            source_table: baseTable,
            column_name: entityColumn.name,
          },
        ],
        latest_by: {
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: timestampColumn.name,
          direction: 'desc',
        },
      };
    } else if (nextTemplate === 'counts-per-day' && timestampColumn) {
      const countTarget = entityColumn ?? timestampColumn;
      columns = [
        {
          kind: 'derived',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: timestampColumn.name,
          alias: `${timestampColumn.name}_day`,
          aggregate: null,
          derived: {
            alias: `${timestampColumn.name}_day`,
            operation: 'date_bucket',
            inputs: [
              {
                kind: 'column',
                source_id: null,
                source_schema: baseSchema,
                source_table: baseTable,
                column_name: timestampColumn.name,
              },
            ],
            options: { bucket: 'day' },
          },
        },
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: countTarget.name,
          alias: 'row_count',
          aggregate: 'COUNT',
        },
      ];
    } else if (nextTemplate === 'top-n-per-group' && entityColumn && numericColumn) {
      columns = [
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: entityColumn.name,
          alias: entityColumn.name,
          aggregate: null,
        },
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: (labelColumn ?? numericColumn).name,
          alias: labelColumn?.name ?? numericColumn.name,
          aggregate: null,
        },
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: numericColumn.name,
          alias: numericColumn.name,
          aggregate: null,
        },
      ];
      ranking = {
        partition_by: [
          {
            source_id: null,
            source_schema: baseSchema,
            source_table: baseTable,
            column_name: entityColumn.name,
          },
        ],
        order_by: {
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: numericColumn.name,
          direction: 'desc',
        },
        limit: 3,
      };
    } else if (nextTemplate === 'totals-by-week' && timestampColumn && numericColumn) {
      columns = [
        {
          kind: 'derived',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: timestampColumn.name,
          alias: `${timestampColumn.name}_week`,
          aggregate: null,
          derived: {
            alias: `${timestampColumn.name}_week`,
            operation: 'date_bucket',
            inputs: [
              {
                kind: 'column',
                source_id: null,
                source_schema: baseSchema,
                source_table: baseTable,
                column_name: timestampColumn.name,
              },
            ],
            options: { bucket: 'week' },
          },
        },
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: numericColumn.name,
          alias: `${numericColumn.name}_total`,
          aggregate: 'SUM',
        },
      ];
    } else if (nextTemplate === 'previous-row-delta' && entityColumn && timestampColumn && numericColumn) {
      const prevSource: ViewSourceRef = {
        id: 'self-previous',
        kind: 'self',
        schema: baseSchema,
        table: baseTable,
        label: 'This table again (previous row)',
        self: {
          entity_column: entityColumn.name,
          order_column: timestampColumn.name,
          direction: 'previous',
        },
      };
      sources = [prevSource];
      columns = [
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: entityColumn.name,
          alias: entityColumn.name,
          aggregate: null,
        },
        {
          kind: 'source',
          source_id: null,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: numericColumn.name,
          alias: numericColumn.name,
          aggregate: null,
        },
        {
          kind: 'derived',
          source_id: prevSource.id,
          source_schema: baseSchema,
          source_table: baseTable,
          column_name: numericColumn.name,
          alias: `${numericColumn.name}_delta`,
          aggregate: null,
          derived: {
            alias: `${numericColumn.name}_delta`,
            operation: 'difference',
            inputs: [
              {
                kind: 'column',
                source_id: null,
                source_schema: baseSchema,
                source_table: baseTable,
                column_name: numericColumn.name,
              },
              {
                kind: 'column',
                source_id: prevSource.id,
                source_schema: baseSchema,
                source_table: baseTable,
                column_name: numericColumn.name,
              },
            ],
            options: null,
          },
        },
      ];
    }

    templateStage = false;
  }

  function handleBaseTableChange(value: string) {
    const [schema, table] = value.split('.');
    baseSchema = schema ?? '';
    baseTable = table ?? '';
    columns = [];
    filters = {};
    sources = [];
    grouping = null;
    ranking = null;
    template = templateStage ? template : null;
    error = '';
  }

  function openPicker(index: number | null) {
    pickerIndex = index;
    pickerValue = index == null ? null : cloneColumn(columns[index]);
    pickerOpen = true;
  }

  function pruneUnusedSources(nextColumns: ViewColumn[]) {
    const referenced = new Set<string>();
    for (const column of nextColumns) {
      if (column.source_id) referenced.add(column.source_id);
      for (const input of column.derived?.inputs ?? []) {
        if (input.source_id) referenced.add(input.source_id);
      }
    }
    sources = sources.filter((source) => referenced.has(source.id));
  }

  function handlePickerSave(payload: PickerPayload) {
    let nextColumns = [...columns];
    if (pickerIndex == null) {
      nextColumns = [...nextColumns, payload.column];
    } else {
      nextColumns[pickerIndex] = payload.column;
    }

    if (payload.source) {
      const existingIndex = sources.findIndex((source) => source.id === payload.source?.id);
      if (existingIndex >= 0) {
        sources = sources.map((source, index) => (index === existingIndex ? payload.source! : source));
      } else {
        sources = [...sources, payload.source];
      }
    }

    columns = nextColumns;
    pruneUnusedSources(nextColumns);
    pickerOpen = false;
    pickerIndex = null;
    pickerValue = null;
  }

  function removeColumn(index: number) {
    const nextColumns = columns.filter((_, currentIndex) => currentIndex !== index);
    columns = nextColumns;
    pruneUnusedSources(nextColumns);
  }

  function moveColumn(index: number, direction: -1 | 1) {
    const nextIndex = index + direction;
    if (nextIndex < 0 || nextIndex >= columns.length) return;
    const nextColumns = [...columns];
    const [moved] = nextColumns.splice(index, 1);
    nextColumns.splice(nextIndex, 0, moved);
    columns = nextColumns;
  }

  function updateFilter(key: string, nextValue: ViewFilterValue | null) {
    if (!nextValue) {
      const nextFilters = { ...filters };
      delete nextFilters[key];
      filters = nextFilters;
      return;
    }
    filters = {
      ...filters,
      [key]: nextValue,
    };
  }

  function setFilterOperator(key: string, operator: ViewFilterValue['op']) {
    if (operator === 'between') {
      updateFilter(key, { op: 'between', value: ['', ''] });
      return;
    }
    if (operator === 'is_empty') {
      updateFilter(key, { op: 'is_empty' });
      return;
    }
    if (operator === 'in_list') {
      updateFilter(key, { op: 'in_list', value: [] });
      return;
    }
    updateFilter(key, { op: operator, value: '' });
  }

  function setFilterPrimaryValue(key: string, value: string) {
    const current = filters[key];
    if (!current) return;
    if (current.op === 'between') {
      updateFilter(key, { ...current, value: [value, current.value[1]] });
      return;
    }
    if (current.op === 'in_list') {
      updateFilter(key, {
        ...current,
        value: value
          .split(',')
          .map((item) => item.trim())
          .filter(Boolean),
      });
      return;
    }
    if (current.op === 'is_empty') return;
    updateFilter(key, { ...current, value });
  }

  function setFilterSecondaryValue(key: string, value: string) {
    const current = filters[key];
    if (!current || current.op !== 'between') return;
    updateFilter(key, { ...current, value: [current.value[0], value] });
  }

  function handleGroupingChange(value: string) {
    const ref = decodeColumnRef(value);
    if (!ref) {
      grouping = null;
      return;
    }
    grouping = {
      keys: [ref],
      latest_by: grouping?.latest_by ?? null,
    };
  }

  function handleLatestByChange(value: string) {
    const latestBy = toOrderBy(value, 'desc');
    if (!latestBy) {
      grouping = grouping ? { ...grouping, latest_by: null } : null;
      return;
    }
    grouping = {
      keys: grouping?.keys?.length ? grouping.keys : [latestBy],
      latest_by: latestBy,
    };
  }

  function handleToggleRanking() {
    if (ranking) {
      ranking = null;
      return;
    }
    const fallback = decodeColumnRef(currentGroupingValue) ?? decodeColumnRef(fieldOptions[0]?.value ?? '');
    ranking = fallback
      ? {
          partition_by: [fallback],
          order_by: toOrderBy(currentRankValue || fieldOptions[0]?.value || '', 'desc'),
          limit: 3,
        }
      : null;
  }

  function handleRankLimitChange(value: number) {
    if (!ranking) return;
    ranking = {
      ...ranking,
      limit: Number.isFinite(value) && value > 0 ? value : ranking.limit,
    };
  }

  function handleRankByChange(value: string) {
    if (!ranking) return;
    ranking = {
      ...ranking,
      order_by: toOrderBy(value, 'desc'),
    };
  }

  async function handleSave() {
    if ((initialDraft?.definition_version ?? 2) < 2 && !upgradeConfirmOpen) {
      upgradeConfirmOpen = true;
      return;
    }
    upgradeConfirmOpen = false;
    await doSave();
  }

  async function doSave() {
    const trimmedName = name.trim();
    if (!trimmedName) {
      error = 'Saved view name must not be empty';
      return;
    }
    if (!baseSchema || !baseTable) {
      error = 'Choose a base table first';
      return;
    }
    if (columns.length === 0) {
      error = 'Add at least one output column';
      return;
    }

    saving = true;
    error = '';
    try {
      const saved = await createView({
        ...buildDraft(),
        name: trimmedName,
      });
      await onSaved(saved);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to save view';
    } finally {
      saving = false;
    }
  }
</script>

{#if templateStage}
  <ViewBuilderTemplates
    {tables}
    baseValue={`${baseSchema}.${baseTable}`}
    onBaseChange={handleBaseTableChange}
    onSelect={(nextTemplate) => void seedTemplate(nextTemplate)}
    onCancel={onCancel}
  />
{:else}
  <div class="builder">
    <ViewBuilderTopBar
      {tables}
      {sourceLabel}
      {templateLabel}
      {saving}
      name={name}
      baseValue={`${baseSchema}.${baseTable}`}
      groupingKey={currentGroupingValue}
      groupingOptions={fieldOptions}
      latestBy={currentLatestValue}
      latestOptions={latestOptions}
      rankBy={currentRankValue}
      rankingEnabled={ranking != null}
      rankLimit={ranking?.limit ?? 3}
      filterCount={currentFilterCount}
      onNameChange={(value) => (name = value)}
      onBaseChange={handleBaseTableChange}
      onGroupingChange={handleGroupingChange}
      onLatestByChange={handleLatestByChange}
      onRankByChange={handleRankByChange}
      onToggleRanking={handleToggleRanking}
      onRankLimitChange={handleRankLimitChange}
      onToggleFilters={() => (showFilters = !showFilters)}
      onSave={() => void handleSave()}
      onCancel={onCancel}
    />

    {#if showFilters}
      <section class="filter-panel" data-testid="view-builder-filter-panel">
        <div class="filter-panel__header">
          <div>
            <p class="eyebrow">Definition filters</p>
            <h3>Store these rules with the view</h3>
          </div>
        </div>

        {#if filterRows.length > 0}
          <div class="filter-panel__rows">
            {#each filterRows as row (row.key)}
              <div class="filter-row">
                <strong>{row.label}</strong>
                <select
                  value={row.filter?.op ?? ''}
                  onchange={(event) => {
                    const operator = (event.currentTarget as HTMLSelectElement).value as ViewFilterValue['op'];
                    if (!operator) {
                      updateFilter(row.key, null);
                    } else {
                      setFilterOperator(row.key, operator);
                    }
                  }}
                >
                  <option value="">No filter</option>
                  <option value="eq">Equals</option>
                  <option value="contains">Contains</option>
                  <option value="starts_with">Starts with</option>
                  <option value="gt">Greater than</option>
                  <option value="gte">Greater than or equal</option>
                  <option value="lt">Less than</option>
                  <option value="lte">Less than or equal</option>
                  <option value="between">Between</option>
                  <option value="is_empty">Is empty</option>
                  <option value="in_list">In list</option>
                </select>

                {#if row.filter?.op === 'between'}
                  <input
                    type="text"
                    value={row.filter.value[0]}
                    placeholder="Start"
                    oninput={(event) => setFilterPrimaryValue(row.key, (event.currentTarget as HTMLInputElement).value)}
                  />
                  <input
                    type="text"
                    value={row.filter.value[1]}
                    placeholder="End"
                    oninput={(event) => setFilterSecondaryValue(row.key, (event.currentTarget as HTMLInputElement).value)}
                  />
                {:else if row.filter?.op === 'in_list'}
                  <input
                    type="text"
                    value={row.filter.value.join(', ')}
                    placeholder="one, two, three"
                    oninput={(event) => setFilterPrimaryValue(row.key, (event.currentTarget as HTMLInputElement).value)}
                  />
                {:else if row.filter?.op !== 'is_empty'}
                  <input
                    type="text"
                    value={'value' in (row.filter ?? {}) ? row.filter?.value ?? '' : ''}
                    placeholder="Value"
                    oninput={(event) => setFilterPrimaryValue(row.key, (event.currentTarget as HTMLInputElement).value)}
                  />
                {:else}
                  <span class="filter-row__hint">Rows with no value in this output.</span>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="filter-panel__empty">Add a column first, then store filters against its output name.</div>
        {/if}
      </section>
    {/if}

    <ViewBuilderGrid
      items={builderItems}
      {preview}
      {previewLoading}
      {error}
      onAdd={() => openPicker(null)}
      onEdit={openPicker}
      onRemove={removeColumn}
      onMove={moveColumn}
    />

    <ColumnPicker
      open={pickerOpen}
      {tables}
      {reachableTables}
      {sources}
      baseSchema={baseSchema}
      baseTable={baseTable}
      value={pickerValue}
      onSave={handlePickerSave}
      onClose={() => {
        pickerOpen = false;
        pickerIndex = null;
        pickerValue = null;
      }}
    />
  </div>
{/if}

{#if upgradeConfirmOpen}
  <div
    class="dialog-backdrop"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) upgradeConfirmOpen = false; }}
  >
    <div class="dialog-card" role="dialog" aria-modal="true" aria-label="Upgrade view definition">
      <p class="dialog-title">Upgrade to version 2?</p>
      <p class="dialog-detail">Saving this draft upgrades the saved-view definition to version 2. This cannot be undone.</p>
      <div class="dialog-actions">
        <button type="button" class="dialog-btn dialog-btn-secondary" onclick={() => upgradeConfirmOpen = false}>Cancel</button>
        <button type="button" class="dialog-btn dialog-btn-primary" onclick={handleSave} data-testid="upgrade-dialog-confirm">Upgrade &amp; save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .builder {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
    min-height: 0;
    padding: var(--sk-space-lg) var(--sk-space-2xl);
    overflow: auto;
  }

  .filter-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-md);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-xl);
    background: rgba(255, 255, 255, 0.78);
    box-shadow: var(--sk-shadow-card);
    padding: var(--sk-space-lg);
  }

  .filter-panel__header h3 {
    margin: 0;
  }

  .eyebrow {
    margin: 0 0 var(--sk-space-xs);
    color: var(--sk-accent);
    font-size: var(--sk-font-size-xs);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .filter-panel__rows {
    display: grid;
    gap: var(--sk-space-md);
  }

  .filter-row {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) repeat(3, minmax(0, 1fr));
    gap: var(--sk-space-sm);
    align-items: center;
  }

  .filter-row select,
  .filter-row input {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.84);
    color: var(--sk-text);
    font: inherit;
    padding: 8px 12px;
  }

  .filter-row__hint,
  .filter-panel__empty {
    color: var(--sk-secondary-strong);
  }

  @media (max-width: 1100px) {
    .builder {
      padding-inline: var(--sk-space-lg);
    }

    .filter-row {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sk-space-lg);
    z-index: 1000;
    animation: dialog-fade 120ms ease-out;
  }

  .dialog-card {
    max-width: 400px;
    width: 100%;
    background: var(--sk-bg, rgba(255, 255, 255, 0.98));
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-lg);
    box-shadow: 0 20px 60px rgba(15, 23, 42, 0.35);
    animation: dialog-pop 140ms ease-out;
  }

  .dialog-title {
    margin: 0 0 var(--sk-space-sm);
    font-weight: 600;
    color: var(--sk-text);
  }

  .dialog-detail {
    margin: 0 0 var(--sk-space-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-muted);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sk-space-sm);
  }

  .dialog-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: var(--sk-radius-md);
    font: inherit;
    cursor: pointer;
    padding: 7px 14px;
    font-size: var(--sk-font-size-body);
  }

  .dialog-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dialog-btn-secondary {
    border: 1px solid var(--sk-border-light);
    background: transparent;
    color: var(--sk-text);
  }

  .dialog-btn-secondary:hover:not(:disabled) {
    background: rgba(47, 72, 88, 0.04);
  }

  .dialog-btn-primary {
    border: 1px solid rgba(0, 169, 165, 0.3);
    background: rgba(0, 169, 165, 0.1);
    color: var(--sk-accent);
  }

  .dialog-btn-primary:hover:not(:disabled) {
    background: rgba(0, 169, 165, 0.18);
  }

  @keyframes dialog-fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes dialog-pop {
    from { transform: scale(0.95); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }
</style>
