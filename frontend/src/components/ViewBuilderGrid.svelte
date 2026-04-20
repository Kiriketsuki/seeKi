<script lang="ts">
  import { ArrowLeft, ArrowRight, Pencil, Plus, Trash2 } from 'lucide-svelte';
  import DataGrid from './DataGrid.svelte';
  import type { QueryResult } from '../lib/types';

  export type BuilderGridItem = {
    id: string;
    label: string;
    detail: string;
    kind: 'source' | 'derived';
  };

  let {
    items = [],
    preview = null,
    previewLoading = false,
    error = '',
    onAdd,
    onEdit,
    onRemove,
    onMove,
  }: {
    items: BuilderGridItem[];
    preview: QueryResult | null;
    previewLoading?: boolean;
    error?: string;
    onAdd: () => void;
    onEdit: (index: number) => void;
    onRemove: (index: number) => void;
    onMove: (index: number, direction: -1 | 1) => void;
  } = $props();
</script>

<section class="builder-grid-shell">
  <div class="builder-grid-shell__header">
    <div>
      <p class="eyebrow">Columns</p>
      <h3>Build directly from the header row</h3>
      <p>Click an empty slot to add a column, or open an existing one to change it.</p>
    </div>
    {#if previewLoading}
      <span class="loading-chip">Refreshing preview…</span>
    {/if}
  </div>

  <div class="column-slots" data-testid="view-builder-grid-slots">
    {#each items as item, index (item.id)}
      <div class="column-slot column-slot--filled" data-testid={`view-builder-slot-${index}`}>
        <button type="button" class="column-slot__button" onclick={() => onEdit(index)}>
          <span class="column-slot__kind">{item.kind === 'derived' ? 'Derived' : 'Column'}</span>
          <strong>{item.label}</strong>
          <span>{item.detail}</span>
        </button>
        <div class="column-slot__actions">
          <button type="button" class="icon-btn" aria-label={`Move ${item.label} left`} onclick={() => onMove(index, -1)} disabled={index === 0}>
            <ArrowLeft size={14} />
          </button>
          <button type="button" class="icon-btn" aria-label={`Move ${item.label} right`} onclick={() => onMove(index, 1)} disabled={index === items.length - 1}>
            <ArrowRight size={14} />
          </button>
          <button type="button" class="icon-btn" aria-label={`Edit ${item.label}`} onclick={() => onEdit(index)}>
            <Pencil size={14} />
          </button>
          <button type="button" class="icon-btn icon-btn--danger" aria-label={`Remove ${item.label}`} onclick={() => onRemove(index)}>
            <Trash2 size={14} />
          </button>
        </div>
      </div>
    {/each}

    <button
      type="button"
      class="column-slot column-slot--empty"
      data-testid="view-builder-add-slot"
      onclick={onAdd}
    >
      <Plus size={16} />
      <span>Add column</span>
    </button>
  </div>

  <div class="preview-shell">
    {#if error}
      <div class="empty-state empty-state--error">{error}</div>
    {:else if preview && preview.columns.length > 0}
      <DataGrid columns={preview.columns} rows={preview.rows} />
    {:else}
      <div class="empty-state">
        {#if items.length === 0}
          Add a column to start previewing the result.
        {:else if previewLoading}
          Running preview…
        {:else}
          No preview rows yet.
        {/if}
      </div>
    {/if}
  </div>
</section>

<style>
  .builder-grid-shell {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
    min-height: 0;
  }

  .builder-grid-shell__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-lg);
  }

  .builder-grid-shell__header h3,
  .builder-grid-shell__header p {
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

  .builder-grid-shell__header p:last-child {
    margin-top: var(--sk-space-sm);
    color: var(--sk-secondary-strong);
  }

  .column-slots {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--sk-space-md);
  }

  .column-slot {
    min-height: 148px;
    border-radius: var(--sk-radius-xl);
    border: 1px solid var(--sk-border-light);
    background: rgba(255, 255, 255, 0.78);
    box-shadow: var(--sk-shadow-card);
  }

  .column-slot--filled {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    overflow: hidden;
  }

  .column-slot__button,
  .column-slot--empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sk-space-sm);
    width: 100%;
    height: 100%;
    border: none;
    background: transparent;
    color: var(--sk-text);
    cursor: pointer;
    padding: var(--sk-space-lg);
    text-align: left;
  }

  .column-slot__kind {
    color: var(--sk-accent);
    font-size: var(--sk-font-size-xs);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .column-slot__button span:last-child {
    color: var(--sk-secondary-strong);
    line-height: 1.45;
  }

  .column-slot__actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    padding: 0 var(--sk-space-md) var(--sk-space-md);
  }

  .column-slot--empty {
    align-items: center;
    justify-content: center;
    border: 1px dashed rgba(0, 169, 165, 0.28);
    background: rgba(0, 169, 165, 0.05);
    color: var(--sk-accent);
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: 1px solid transparent;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.72);
    color: var(--sk-secondary-strong);
    cursor: pointer;
  }

  .icon-btn:hover:not(:disabled) {
    border-color: var(--sk-border-light);
  }

  .icon-btn:disabled {
    cursor: not-allowed;
    opacity: 0.4;
  }

  .icon-btn--danger {
    color: #b54747;
  }

  .preview-shell {
    min-height: 420px;
  }

  .empty-state {
    min-height: 420px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--sk-border-light);
    border-radius: var(--sk-radius-xl);
    background: rgba(255, 255, 255, 0.66);
    color: var(--sk-secondary-strong);
    text-align: center;
    padding: var(--sk-space-xl);
  }

  .empty-state--error {
    color: #b54747;
  }

  .loading-chip {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    background: rgba(0, 169, 165, 0.1);
    color: var(--sk-accent);
    font-size: var(--sk-font-size-xs);
    font-weight: 700;
    padding: 6px 10px;
  }
</style>
