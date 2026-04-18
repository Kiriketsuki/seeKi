<script lang="ts">
  import { ChevronLeft, ChevronRight, X } from 'lucide-svelte';
  import { fetchColumnSamples, fetchColumns } from '../lib/api';
  import type { ColumnInfo, TableInfo, ViewColumn, ViewDerivedColumn, ViewSourceRef } from '../lib/types';

  type PickerMode = 'base' | 'fk' | 'match' | 'self';
  type PickerOperation =
    | 'raw'
    | 'SUM'
    | 'AVG'
    | 'COUNT'
    | 'MIN'
    | 'MAX'
    | 'LATEST'
    | 'difference'
    | 'ratio_percent'
    | 'date_bucket_day'
    | 'date_bucket_week'
    | 'date_part_year'
    | 'date_part_month'
    | 'date_part_weekday'
    | 'age_of_timestamp'
    | 'text_length';

  type PickerPayload = {
    column: ViewColumn;
    source: ViewSourceRef | null;
  };

  type OperationOption = {
    id: PickerOperation;
    label: string;
    detail: string;
    disabled: boolean;
  };

  let {
    open = false,
    tables = [],
    baseSchema = '',
    baseTable = '',
    reachableTables = [],
    sources = [],
    value = null,
    onSave,
    onClose,
  }: {
    open?: boolean;
    tables: TableInfo[];
    baseSchema: string;
    baseTable: string;
    reachableTables: TableInfo[];
    sources: ViewSourceRef[];
    value?: ViewColumn | null;
    onSave: (payload: PickerPayload) => void;
    onClose: () => void;
  } = $props();

  let step = $state(1);
  let mode = $state<PickerMode>('base');
  let selectedSchema = $state('');
  let selectedTable = $state('');
  let selectedColumn = $state('');
  let selectedBaseMatchColumn = $state('');
  let selectedSourceMatchColumn = $state('');
  let selfEntityColumn = $state('');
  let selfOrderColumn = $state('');
  let selfDirection = $state<'previous' | 'next'>('previous');
  let selectedOperation = $state<PickerOperation>('raw');
  let alias = $state('');
  let error = $state('');
  let loadingColumns = $state(false);
  let loadingSamples = $state(false);
  let availableColumns = $state<ColumnInfo[]>([]);
  let baseColumns = $state<ColumnInfo[]>([]);
  let samples = $state<string[]>([]);
  let activeSourceId = $state<string | null>(null);
  let loadKey = $state('');
  let sampleKey = $state('');

  function isNumeric(dataType: string): boolean {
    return ['smallint', 'integer', 'bigint', 'numeric', 'real', 'double precision'].includes(dataType);
  }

  function isTemporal(dataType: string): boolean {
    return ['date', 'timestamp', 'timestamp without time zone', 'timestamp with time zone', 'timestamptz'].includes(dataType);
  }

  function isTextual(dataType: string): boolean {
    return ['text', 'varchar', 'character varying', 'char', 'character', 'citext'].includes(dataType);
  }

  function isComparable(dataType: string): boolean {
    return dataType !== 'json' && dataType !== 'jsonb';
  }

  function parseOperationFromValue(nextValue: ViewColumn | null): PickerOperation {
    if (!nextValue) return 'raw';
    if (nextValue.derived) {
      const operation = nextValue.derived.operation;
      if (operation === 'difference') return 'difference';
      if (operation === 'ratio_percent') return 'ratio_percent';
      if (operation === 'age_of_timestamp') return 'age_of_timestamp';
      if (operation === 'text_length') return 'text_length';
      if (operation === 'date_bucket') {
        return nextValue.derived.options?.bucket === 'week' ? 'date_bucket_week' : 'date_bucket_day';
      }
      if (operation === 'date_part') {
        if (nextValue.derived.options?.part === 'month') return 'date_part_month';
        if (nextValue.derived.options?.part === 'weekday') return 'date_part_weekday';
        return 'date_part_year';
      }
    }
    return (nextValue.aggregate as PickerOperation | null) ?? 'raw';
  }

  function inferMode(nextValue: ViewColumn | null): PickerMode {
    if (!nextValue?.source_id) return 'base';
    const source = sources.find((candidate) => candidate.id === nextValue.source_id);
    if (!source) return 'base';
    if (source.kind === 'self') return 'self';
    if (source.kind === 'match') return 'match';
    return 'fk';
  }

  function qualifiedValue(schema: string, table: string): string {
    return `${schema}.${table}`;
  }

  async function loadTableColumns(schema: string, table: string, kind: 'base' | 'selected') {
    if (!schema || !table) return;
    const key = `${kind}:${schema}.${table}`;
    if (kind === 'selected' && key === loadKey) return;

    try {
      if (kind === 'selected') {
        loadingColumns = true;
        error = '';
      }
      const nextColumns = await fetchColumns(schema, table);
      if (kind === 'base') {
        baseColumns = nextColumns;
      } else {
        availableColumns = nextColumns;
        loadKey = key;
        if (!nextColumns.some((column) => column.name === selectedColumn)) {
          selectedColumn = nextColumns[0]?.name ?? '';
        }
      }
    } catch (err) {
      if (kind === 'selected') {
        error = err instanceof Error ? err.message : 'Failed to load columns';
        availableColumns = [];
        selectedColumn = '';
      }
    } finally {
      if (kind === 'selected') loadingColumns = false;
    }
  }

  async function loadColumnSamples(schema: string, table: string, column: string) {
    if (!schema || !table || !column) return;
    const key = `${schema}.${table}.${column}`;
    if (key === sampleKey) return;
    loadingSamples = true;
    try {
      samples = await fetchColumnSamples(schema, table, column);
      sampleKey = key;
    } catch {
      samples = [];
    } finally {
      loadingSamples = false;
    }
  }

  function seedPicker() {
    const existingSource = value?.source_id
      ? sources.find((candidate) => candidate.id === value.source_id) ?? null
      : null;
    mode = inferMode(value);
    selectedSchema = value?.source_schema ?? existingSource?.schema ?? baseSchema;
    selectedTable = value?.source_table ?? existingSource?.table ?? baseTable;
    selectedColumn = value?.column_name ?? '';
    selectedOperation = parseOperationFromValue(value);
    alias = value?.derived?.alias ?? value?.alias ?? '';
    activeSourceId = existingSource?.id ?? null;
    selectedBaseMatchColumn = existingSource?.match?.base_column ?? '';
    selectedSourceMatchColumn = existingSource?.match?.source_column ?? '';
    selfEntityColumn = existingSource?.self?.entity_column ?? '';
    selfOrderColumn = existingSource?.self?.order_column ?? '';
    selfDirection = existingSource?.self?.direction ?? 'previous';
    availableColumns = [];
    baseColumns = [];
    samples = [];
    loadKey = '';
    sampleKey = '';
    error = '';
    step = 1;
  }

  $effect(() => {
    if (!open) return;
    seedPicker();
  });

  $effect(() => {
    if (!open || !baseSchema || !baseTable) return;
    void loadTableColumns(baseSchema, baseTable, 'base');
  });

  $effect(() => {
    if (!open || !selectedSchema || !selectedTable) return;
    void loadTableColumns(selectedSchema, selectedTable, 'selected');
  });

  $effect(() => {
    if (!open || !selectedSchema || !selectedTable || !selectedColumn) return;
    void loadColumnSamples(selectedSchema, selectedTable, selectedColumn);
  });

  const sameSchemaTables = $derived.by(() =>
    tables.filter((table) => table.schema === baseSchema && table.name !== baseTable),
  );

  const fkChoices = $derived.by(() =>
    reachableTables.filter((table) => !(table.schema === baseSchema && table.name === baseTable)),
  );

  const selectedColumnMeta = $derived.by(
    () => availableColumns.find((column) => column.name === selectedColumn) ?? null,
  );

  const operationOptions = $derived.by(() => {
    const meta = selectedColumnMeta;
    const numeric = meta ? isNumeric(meta.data_type) : false;
    const temporal = meta ? isTemporal(meta.data_type) : false;
    const textual = meta ? isTextual(meta.data_type) : false;
    const comparable = meta ? isComparable(meta.data_type) : false;

    const options: OperationOption[] = [
      { id: 'raw', label: 'Keep value', detail: 'Show the selected value as-is.', disabled: !meta },
      { id: 'COUNT', label: 'Count rows', detail: 'Count how many rows match.', disabled: !meta },
      { id: 'SUM', label: 'Total', detail: 'Add up numeric values.', disabled: !numeric },
      { id: 'AVG', label: 'Average', detail: 'Average numeric values.', disabled: !numeric },
      { id: 'MIN', label: 'Smallest', detail: 'Keep the smallest value.', disabled: !comparable },
      { id: 'MAX', label: 'Largest', detail: 'Keep the largest value.', disabled: !comparable },
      { id: 'LATEST', label: 'Latest value', detail: 'Use the newest value once Latest by is set.', disabled: !meta },
      { id: 'difference', label: 'Difference', detail: 'Compare this row with the previous or next row.', disabled: !(mode === 'self' && numeric) },
      { id: 'ratio_percent', label: 'Ratio %', detail: 'Express the change as a percent.', disabled: !(mode === 'self' && numeric) },
      { id: 'date_bucket_day', label: 'Day bucket', detail: 'Group timestamps into days.', disabled: !temporal },
      { id: 'date_bucket_week', label: 'Week bucket', detail: 'Group timestamps into weeks.', disabled: !temporal },
      { id: 'date_part_year', label: 'Year', detail: 'Pull out the year number.', disabled: !temporal },
      { id: 'date_part_month', label: 'Month', detail: 'Pull out the month number.', disabled: !temporal },
      { id: 'date_part_weekday', label: 'Weekday', detail: 'Pull out the weekday number.', disabled: !temporal },
      { id: 'age_of_timestamp', label: 'Age', detail: 'Show how old each timestamp is.', disabled: !temporal },
      { id: 'text_length', label: 'Text length', detail: 'Count the characters in the text.', disabled: !textual },
    ];

    if (options.every((option) => option.id !== selectedOperation || !option.disabled)) {
      return options;
    }
    selectedOperation = 'raw';
    return options;
  });

  function buildSourceDefinition(): ViewSourceRef | null {
    if (mode === 'base') return null;
    if (mode === 'self') {
      return {
        id: activeSourceId ?? `self-${selfDirection}`,
        kind: 'self',
        schema: baseSchema,
        table: baseTable,
        label: selfDirection === 'previous' ? 'This table again (previous row)' : 'This table again (next row)',
        self: {
          entity_column: selfEntityColumn,
          order_column: selfOrderColumn,
          direction: selfDirection,
        },
      };
    }
    if (mode === 'match') {
      return {
        id: activeSourceId ?? `match-${selectedSchema}.${selectedTable}`,
        kind: 'match',
        schema: selectedSchema,
        table: selectedTable,
        label: `Match ${selectedSchema}.${selectedTable}`,
        match: {
          base_column: selectedBaseMatchColumn,
          source_column: selectedSourceMatchColumn,
        },
      };
    }
    return {
      id: activeSourceId ?? `fk-${selectedSchema}.${selectedTable}`,
      kind: 'fk',
      schema: selectedSchema,
      table: selectedTable,
      label: `${selectedSchema}.${selectedTable}`,
    };
  }

  function defaultAliasForOperation(): string {
    if (!selectedColumn) return '';
    switch (selectedOperation) {
      case 'difference':
        return `${selectedColumn}_delta`;
      case 'ratio_percent':
        return `${selectedColumn}_ratio_pct`;
      case 'date_bucket_day':
        return `${selectedColumn}_day`;
      case 'date_bucket_week':
        return `${selectedColumn}_week`;
      case 'date_part_year':
        return `${selectedColumn}_year`;
      case 'date_part_month':
        return `${selectedColumn}_month`;
      case 'date_part_weekday':
        return `${selectedColumn}_weekday`;
      case 'age_of_timestamp':
        return `${selectedColumn}_age`;
      case 'text_length':
        return `${selectedColumn}_length`;
      default:
        return '';
    }
  }

  function buildDerived(): ViewDerivedColumn | null {
    const derivedAlias = alias.trim() || defaultAliasForOperation();
    const primaryInput = {
      kind: 'column' as const,
      source_id: mode === 'base' ? null : buildSourceDefinition()?.id ?? null,
      source_schema: selectedSchema,
      source_table: selectedTable,
      column_name: selectedColumn,
    };

    switch (selectedOperation) {
      case 'difference':
      case 'ratio_percent':
        return {
          alias: derivedAlias,
          operation: selectedOperation,
          inputs: [
            {
              kind: 'column',
              source_id: null,
              source_schema: baseSchema,
              source_table: baseTable,
              column_name: selectedColumn,
            },
            primaryInput,
          ],
          options: null,
        };
      case 'date_bucket_day':
        return {
          alias: derivedAlias,
          operation: 'date_bucket',
          inputs: [primaryInput],
          options: { bucket: 'day' },
        };
      case 'date_bucket_week':
        return {
          alias: derivedAlias,
          operation: 'date_bucket',
          inputs: [primaryInput],
          options: { bucket: 'week' },
        };
      case 'date_part_year':
        return {
          alias: derivedAlias,
          operation: 'date_part',
          inputs: [primaryInput],
          options: { part: 'year' },
        };
      case 'date_part_month':
        return {
          alias: derivedAlias,
          operation: 'date_part',
          inputs: [primaryInput],
          options: { part: 'month' },
        };
      case 'date_part_weekday':
        return {
          alias: derivedAlias,
          operation: 'date_part',
          inputs: [primaryInput],
          options: { part: 'weekday' },
        };
      case 'age_of_timestamp':
        return {
          alias: derivedAlias,
          operation: 'age_of_timestamp',
          inputs: [primaryInput],
          options: null,
        };
      case 'text_length':
        return {
          alias: derivedAlias,
          operation: 'text_length',
          inputs: [primaryInput],
          options: null,
        };
      default:
        return null;
    }
  }

  function buildColumn(): ViewColumn {
    const source = buildSourceDefinition();
    const derived = buildDerived();
    const derivedAlias = derived?.alias ?? null;
    const aggregate = ['SUM', 'AVG', 'COUNT', 'MIN', 'MAX', 'LATEST'].includes(selectedOperation)
      ? selectedOperation
      : null;

    return {
      kind: derived ? 'derived' : 'source',
      source_id: source?.id ?? null,
      source_schema: selectedSchema,
      source_table: selectedTable,
      column_name: selectedColumn,
      alias: derivedAlias ?? (alias.trim() || null),
      aggregate: aggregate as ViewColumn['aggregate'],
      derived,
    };
  }

  function validateStep(currentStep: number): boolean {
    if (currentStep === 1) {
      if (!selectedSchema || !selectedTable) return false;
      if (mode === 'match') return Boolean(selectedBaseMatchColumn && selectedSourceMatchColumn);
      if (mode === 'self') return Boolean(selfEntityColumn && selfOrderColumn);
      return true;
    }
    if (currentStep === 2) return Boolean(selectedColumn);
    if (currentStep === 4 && buildDerived()) return Boolean(alias.trim() || defaultAliasForOperation());
    return true;
  }

  function goToStep(nextStep: number) {
    if (nextStep > step && !validateStep(step)) return;
    step = nextStep;
  }

  function handleSourceMode(nextMode: PickerMode) {
    mode = nextMode;
    activeSourceId = null;
    if (nextMode === 'base' || nextMode === 'self') {
      selectedSchema = baseSchema;
      selectedTable = baseTable;
    } else if (nextMode === 'fk') {
      const fallback = fkChoices[0];
      selectedSchema = fallback?.schema ?? baseSchema;
      selectedTable = fallback?.name ?? baseTable;
    } else {
      const fallback = sameSchemaTables[0];
      selectedSchema = fallback?.schema ?? baseSchema;
      selectedTable = fallback?.name ?? baseTable;
    }
    selectedColumn = '';
    selectedOperation = 'raw';
    samples = [];
    sampleKey = '';
    loadKey = '';
  }

  function handleSave() {
    const source = buildSourceDefinition();
    onSave({
      column: buildColumn(),
      source,
    });
  }
</script>

{#if open}
  <div
    class="picker-backdrop"
    role="presentation"
    onclick={onClose}
  >
    <div
      class="picker"
      role="dialog"
      aria-modal="true"
      aria-label="Choose a view column"
      tabindex="-1"
      data-testid="view-column-picker"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.key === 'Escape' && onClose()}
    >
      <div class="picker__header">
        <div>
          <p class="eyebrow">Column picker</p>
          <h3>{value ? 'Edit output' : 'Add output'}</h3>
          <p>Move through the steps, then save the output back into the header row.</p>
        </div>
        <button type="button" class="icon-btn" aria-label="Close column picker" onclick={onClose}>
          <X size={16} />
        </button>
      </div>

      <div class="picker__steps" data-testid="view-column-picker-steps">
        {#each [1, 2, 3, 4] as stepNumber}
          <button
            type="button"
            class="step-pill"
            class:active={step === stepNumber}
            data-testid={`view-column-picker-step-${stepNumber}`}
            onclick={() => goToStep(stepNumber)}
          >
            Step {stepNumber}
          </button>
        {/each}
      </div>

      {#if step === 1}
        <section class="picker__panel">
          <div>
            <h4>1. Choose where the column comes from</h4>
            <p>Pick the base table, a related table, a manual column match, or this table again for previous-row comparisons.</p>
          </div>

          <div class="mode-grid">
            <button type="button" class="mode-card" class:active={mode === 'base'} onclick={() => handleSourceMode('base')}>Base table</button>
            <button type="button" class="mode-card" class:active={mode === 'fk'} onclick={() => handleSourceMode('fk')}>FK-reachable table</button>
            <button type="button" class="mode-card" class:active={mode === 'match'} onclick={() => handleSourceMode('match')}>Match on a column…</button>
            <button type="button" class="mode-card" class:active={mode === 'self'} onclick={() => handleSourceMode('self')}>This table again…</button>
          </div>

          {#if mode !== 'self'}
            <label class="field">
              <span>{mode === 'base' ? 'Source table' : 'Target table'}</span>
              <select value={`${selectedSchema}.${selectedTable}`} onchange={(event) => {
                const [schema, table] = (event.currentTarget as HTMLSelectElement).value.split('.');
                selectedSchema = schema ?? '';
                selectedTable = table ?? '';
                selectedColumn = '';
                loadKey = '';
              }}>
                {#if mode === 'base'}
                  <option value={`${baseSchema}.${baseTable}`}>{baseSchema}.{baseTable}</option>
                {:else if mode === 'fk'}
                  {#each fkChoices as table (`${table.schema}.${table.name}`)}
                    <option value={`${table.schema}.${table.name}`}>{table.schema}.{table.name}</option>
                  {/each}
                {:else}
                  {#each sameSchemaTables as table (`${table.schema}.${table.name}`)}
                    <option value={`${table.schema}.${table.name}`}>{table.schema}.{table.name}</option>
                  {/each}
                {/if}
              </select>
            </label>
          {/if}

          {#if mode === 'match'}
            <div class="field-grid">
              <label class="field">
                <span>Base-table column</span>
                <select bind:value={selectedBaseMatchColumn}>
                  <option value="">Choose a base column</option>
                  {#each baseColumns as column (column.name)}
                    <option value={column.name}>{column.display_name}</option>
                  {/each}
                </select>
              </label>
              <label class="field">
                <span>Matched column</span>
                <select bind:value={selectedSourceMatchColumn}>
                  <option value="">Choose a matched column</option>
                  {#each availableColumns as column (column.name)}
                    <option value={column.name}>{column.display_name}</option>
                  {/each}
                </select>
              </label>
            </div>
          {/if}

          {#if mode === 'self'}
            <div class="field-grid">
              <label class="field">
                <span>Same entity by</span>
                <select bind:value={selfEntityColumn}>
                  <option value="">Choose a column</option>
                  {#each baseColumns as column (column.name)}
                    <option value={column.name}>{column.display_name}</option>
                  {/each}
                </select>
              </label>
              <label class="field">
                <span>Ordered by</span>
                <select bind:value={selfOrderColumn}>
                  <option value="">Choose a column</option>
                  {#each baseColumns as column (column.name)}
                    <option value={column.name}>{column.display_name}</option>
                  {/each}
                </select>
              </label>
              <label class="field">
                <span>Compare with</span>
                <select bind:value={selfDirection}>
                  <option value="previous">Previous row</option>
                  <option value="next">Next row</option>
                </select>
              </label>
            </div>
          {/if}
        </section>
      {:else if step === 2}
        <section class="picker__panel">
          <div>
            <h4>2. Choose the column and sample values</h4>
            <p>Pick the source column, then scan a few recent distinct values before you use it.</p>
          </div>

          <label class="field">
            <span>Source column</span>
            <select bind:value={selectedColumn} disabled={loadingColumns || availableColumns.length === 0}>
              <option value="">Choose a column</option>
              {#each availableColumns as column (column.name)}
                <option value={column.name}>{column.display_name}</option>
              {/each}
            </select>
          </label>

          {#if selectedColumnMeta}
            <div class="meta-row">
              <span>{selectedColumnMeta.display_name}</span>
              <span>{selectedColumnMeta.display_type}</span>
            </div>
          {/if}

          <div class="sample-box">
            <strong>Sample values</strong>
            {#if loadingSamples}
              <span>Loading samples…</span>
            {:else if samples.length > 0}
              <div class="sample-chips">
                {#each samples as sample (sample)}
                  <span class="sample-chip">{sample}</span>
                {/each}
              </div>
            {:else}
              <span>No sample values found.</span>
            {/if}
          </div>
        </section>
      {:else if step === 3}
        <section class="picker__panel">
          <div>
            <h4>3. Choose how the output behaves</h4>
            <p>Pick a plain-language behavior. Disabled options do not fit the selected data type yet.</p>
          </div>

          <div class="operation-grid">
            {#each operationOptions as option (option.id)}
              <button
                type="button"
                class="operation-chip"
                class:active={selectedOperation === option.id}
                disabled={option.disabled}
                onclick={() => (selectedOperation = option.id)}
              >
                <strong>{option.label}</strong>
                <span>{option.detail}</span>
              </button>
            {/each}
          </div>
        </section>
      {:else}
        <section class="picker__panel">
          <div>
            <h4>4. Choose the output name</h4>
            <p>Use a plain label that will make sense later in the saved view and the browsing grid.</p>
          </div>

          <label class="field">
            <span>Output name</span>
            <input bind:value={alias} type="text" placeholder={defaultAliasForOperation() || 'Optional output name'} />
          </label>

          <div class="summary-box">
            <strong>Summary</strong>
            <span>
              {selectedOperation === 'raw'
                ? `Keep ${selectedTable}.${selectedColumn} as a visible output.`
                : `Create ${selectedOperation.replaceAll('_', ' ')} from ${selectedTable}.${selectedColumn}.`}
            </span>
          </div>
        </section>
      {/if}

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="picker__footer">
        <button type="button" class="secondary" onclick={onClose}>Cancel</button>
        <div class="picker__footer-right">
          <button type="button" class="secondary" onclick={() => goToStep(Math.max(1, step - 1))} disabled={step === 1}>
            <ChevronLeft size={14} />
            <span>Back</span>
          </button>
          {#if step < 4}
            <button type="button" class="primary" onclick={() => goToStep(step + 1)} disabled={!validateStep(step)}>
              <span>Next</span>
              <ChevronRight size={14} />
            </button>
          {:else}
            <button type="button" class="primary" data-testid="view-column-picker-save" onclick={handleSave}>
              Save column
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .picker-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(21, 33, 38, 0.36);
    padding: var(--sk-space-lg);
  }

  .picker {
    width: min(920px, 100%);
    max-height: min(88vh, 860px);
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
    overflow: auto;
    border: 1px solid var(--sk-border-light);
    border-radius: 24px;
    background: rgba(255, 255, 255, 0.96);
    box-shadow: 0 28px 64px rgba(16, 27, 33, 0.24);
    padding: var(--sk-space-xl);
  }

  .picker__header,
  .picker__footer {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-lg);
  }

  .picker__header h3,
  .picker__header p,
  .picker__panel h4,
  .picker__panel p {
    margin: 0;
  }

  .picker__header p:last-child,
  .picker__panel p:last-child {
    margin-top: var(--sk-space-sm);
    color: var(--sk-secondary-strong);
  }

  .eyebrow {
    color: var(--sk-accent);
    font-size: var(--sk-font-size-xs);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .picker__steps {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-sm);
  }

  .step-pill,
  .mode-card,
  .operation-chip,
  .icon-btn,
  .secondary,
  .primary {
    font: inherit;
  }

  .step-pill,
  .secondary,
  .mode-card {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.78);
    color: var(--sk-text);
    cursor: pointer;
    padding: 8px 12px;
  }

  .step-pill.active,
  .mode-card.active {
    border-color: rgba(0, 169, 165, 0.32);
    background: rgba(0, 169, 165, 0.1);
    color: var(--sk-accent);
  }

  .picker__panel {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
  }

  .mode-grid,
  .field-grid,
  .operation-grid {
    display: grid;
    gap: var(--sk-space-md);
  }

  .mode-grid {
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  }

  .field-grid {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  .operation-grid {
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  }

  .mode-card,
  .operation-chip {
    text-align: left;
  }

  .mode-card {
    min-height: 72px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
    color: var(--sk-secondary-strong);
    font-size: var(--sk-font-size-sm);
  }

  .field select,
  .field input {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.84);
    color: var(--sk-text);
    font: inherit;
    padding: 10px 12px;
  }

  .meta-row,
  .sample-box,
  .summary-box {
    display: flex;
    gap: var(--sk-space-sm);
    border-radius: var(--sk-radius-lg);
    background: rgba(47, 72, 88, 0.05);
    color: var(--sk-secondary-strong);
    padding: var(--sk-space-md);
  }

  .sample-box,
  .summary-box {
    flex-direction: column;
  }

  .sample-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-sm);
  }

  .sample-chip {
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.86);
    color: var(--sk-text);
    padding: 6px 10px;
  }

  .operation-chip {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    min-height: 124px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.84);
    color: var(--sk-text);
    cursor: pointer;
    padding: var(--sk-space-md);
  }

  .operation-chip.active {
    border-color: rgba(0, 169, 165, 0.32);
    background: rgba(0, 169, 165, 0.1);
  }

  .operation-chip span {
    color: var(--sk-secondary-strong);
    line-height: 1.45;
  }

  .operation-chip:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }

  .picker__footer-right {
    display: flex;
    gap: var(--sk-space-sm);
  }

  .icon-btn,
  .secondary,
  .primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border-radius: var(--sk-radius-md);
    cursor: pointer;
    padding: 8px 14px;
  }

  .icon-btn {
    border: 1px solid transparent;
    background: transparent;
    color: var(--sk-text);
  }

  .primary {
    border: 1px solid rgba(0, 169, 165, 0.32);
    background: rgba(0, 169, 165, 0.12);
    color: var(--sk-accent);
  }

  .primary:disabled,
  .secondary:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .error {
    margin: 0;
    color: #b54747;
  }
</style>
