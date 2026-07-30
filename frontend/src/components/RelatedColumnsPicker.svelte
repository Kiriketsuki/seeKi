<script lang="ts">
  import { X } from 'lucide-svelte';
  import { fetchColumns } from '../lib/api';
  import type { ColumnInfo, OutgoingRelationship, PickedRelatedColumn, TableRelationships } from '../lib/types';

  let {
    open = false,
    baseSchema = '',
    baseTable = '',
    relationships = null,
    picked = [],
    onSave,
    onClose,
  }: {
    open?: boolean;
    baseSchema: string;
    baseTable: string;
    relationships: TableRelationships | null;
    picked: PickedRelatedColumn[];
    onSave: (next: PickedRelatedColumn[]) => void;
    onClose: () => void;
  } = $props();

  let step = $state<1 | 2>(1);
  let targetSchema = $state('');
  let targetTable = $state('');
  let targetDisplayName = $state('');
  let targetColumns = $state<ColumnInfo[]>([]);
  let checkedColumns = $state<Set<string>>(new Set());
  let loadingColumns = $state(false);
  let loadError = $state('');

  let allowedTargets = $derived.by((): OutgoingRelationship[] =>
    (relationships?.outgoing ?? []).filter((edge) => edge.target.allowed),
  );

  $effect(() => {
    if (open) {
      step = 1;
      targetSchema = '';
      targetTable = '';
      targetDisplayName = '';
      targetColumns = [];
      checkedColumns = new Set();
      loadError = '';
    }
  });

  async function selectTarget(edge: OutgoingRelationship) {
    targetSchema = edge.target.schema;
    targetTable = edge.target.table;
    targetDisplayName = edge.target.display_name;
    loadingColumns = true;
    loadError = '';
    try {
      targetColumns = await fetchColumns(edge.target.schema, edge.target.table);
      checkedColumns = new Set(
        picked
          .filter((p) => p.schema === edge.target.schema && p.table === edge.target.table)
          .map((p) => p.column),
      );
      step = 2;
    } catch (e) {
      loadError = e instanceof Error ? e.message : 'Failed to load related columns';
    } finally {
      loadingColumns = false;
    }
  }

  function toggleColumn(name: string) {
    const next = new Set(checkedColumns);
    if (next.has(name)) {
      next.delete(name);
    } else {
      next.add(name);
    }
    checkedColumns = next;
  }

  function removePicked(schema: string, table: string, column: string) {
    onSave(picked.filter((p) => !(p.schema === schema && p.table === table && p.column === column)));
  }

  function handleSave() {
    const withoutTarget = picked.filter(
      (p) => !(p.schema === targetSchema && p.table === targetTable),
    );
    const nextForTarget: PickedRelatedColumn[] = targetColumns
      .filter((column) => checkedColumns.has(column.name))
      .map((column) => ({
        schema: targetSchema,
        table: targetTable,
        tableDisplayName: targetDisplayName,
        column: column.name,
        columnDisplayName: column.display_name,
      }));
    onSave([...withoutTarget, ...nextForTarget]);
    onClose();
  }
</script>

{#if open}
  <div class="related-backdrop" role="presentation" onclick={onClose}>
    <div
      class="related-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Add related information"
      tabindex="-1"
      data-testid="related-columns-picker"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.key === 'Escape' && onClose()}
    >
      <div class="related-dialog__header">
        <div>
          <p class="eyebrow">Related information</p>
          <h3>Add related information</h3>
          <p>Bring in columns from a linked table, so they show up right in this grid.</p>
        </div>
        <button type="button" class="icon-btn" aria-label="Close" onclick={onClose}>
          <X size={16} />
        </button>
      </div>

      {#if picked.length > 0}
        <div class="picked-summary" data-testid="related-columns-picked">
          <h4>Already added</h4>
          <ul>
            {#each picked as item (`${item.schema}.${item.table}.${item.column}`)}
              <li>
                <span>{item.columnDisplayName} <span class="muted">from {item.tableDisplayName}</span></span>
                <button
                  type="button"
                  class="icon-btn"
                  aria-label={`Remove ${item.columnDisplayName} from ${item.tableDisplayName}`}
                  onclick={() => removePicked(item.schema, item.table, item.column)}
                >
                  <X size={14} />
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      {#if step === 1}
        <section class="related-dialog__panel">
          <h4>1. Choose a related table</h4>
          {#if allowedTargets.length === 0}
            <p>No related tables are available for {baseSchema}.{baseTable}.</p>
          {:else}
            <ul class="target-list">
              {#each allowedTargets as edge (edge.constraint)}
                <li>
                  <button type="button" class="target-card" onclick={() => selectTarget(edge)}>
                    From {edge.target.display_name}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {:else}
        <section class="related-dialog__panel">
          <h4>2. Choose columns from {targetDisplayName}</h4>
          {#if loadingColumns}
            <p>Loading columns…</p>
          {:else if loadError}
            <p class="error">{loadError}</p>
          {:else}
            <ul class="column-list">
              {#each targetColumns as column (column.name)}
                <li>
                  <label class="checkbox-row">
                    <input
                      type="checkbox"
                      checked={checkedColumns.has(column.name)}
                      onchange={() => toggleColumn(column.name)}
                    />
                    <span>{column.display_name}</span>
                  </label>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/if}

      <div class="related-dialog__footer">
        {#if step === 2}
          <button type="button" class="btn-secondary" onclick={() => (step = 1)}>Back</button>
        {/if}
        <div class="spacer"></div>
        <button type="button" class="btn-secondary" onclick={onClose}>Cancel</button>
        {#if step === 2}
          <button type="button" class="btn-primary" onclick={handleSave}>Save</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .related-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(var(--sk-ink-rgb), 0.36);
    backdrop-filter: blur(3px);
    -webkit-backdrop-filter: blur(3px);
    padding: var(--sk-space-lg);
    animation: sk-fade-in 140ms ease-out;
  }

  .related-dialog {
    width: min(560px, 100%);
    max-height: min(88vh, 720px);
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
    overflow: auto;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: var(--sk-glass-popup);
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    box-shadow: var(--sk-shadow-pop);
    padding: var(--sk-space-xl);
  }

  .related-dialog__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-lg);
  }

  .related-dialog__header h3 {
    margin: 0;
    color: var(--sk-text);
  }

  .related-dialog__header p {
    margin: 0;
    color: var(--sk-secondary-strong);
  }

  .eyebrow {
    margin: 0 0 var(--sk-space-xs);
    color: var(--sk-accent-active-strong);
    font-size: var(--sk-font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .related-dialog__panel {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-md);
  }

  .related-dialog__panel h4 {
    margin: 0;
    color: var(--sk-text);
  }

  .picked-summary {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-md);
    background: rgba(var(--sk-ink-rgb), 0.03);
  }

  .picked-summary h4 {
    margin: 0;
    font-size: var(--sk-font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--sk-secondary-strong);
  }

  .picked-summary ul,
  .target-list,
  .column-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
  }

  .picked-summary li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    color: var(--sk-text);
  }

  .muted {
    color: var(--sk-secondary-strong);
    font-size: var(--sk-font-size-sm);
  }

  .target-card {
    width: 100%;
    text-align: left;
    border: 1px solid var(--sk-border-input);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    color: var(--sk-text);
    padding: var(--sk-space-sm) var(--sk-space-md);
    cursor: pointer;
  }

  .target-card:hover {
    border-color: rgba(var(--sk-accent-active-rgb), 0.32);
    background: rgba(var(--sk-accent-active-rgb), 0.1);
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    color: var(--sk-text);
    padding: var(--sk-space-xs) 0;
  }

  .related-dialog__footer {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  .spacer {
    flex: 1;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--sk-secondary-strong);
    cursor: pointer;
    padding: var(--sk-space-xs);
    border-radius: var(--sk-radius-sm);
  }

  .icon-btn:hover {
    background: rgba(var(--sk-ink-rgb), 0.06);
    color: var(--sk-text);
  }

  .btn-primary,
  .btn-secondary {
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font-size: var(--sk-font-size-sm);
    cursor: pointer;
  }

  .btn-primary {
    border: none;
    background: var(--sk-accent-active-strong);
    color: var(--sk-on-accent, #fff);
  }

  .btn-secondary {
    border: 1px solid var(--sk-border-input);
    background: transparent;
    color: var(--sk-text);
  }

  .error {
    color: var(--sk-danger, #c0392b);
  }
</style>
