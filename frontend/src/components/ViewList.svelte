<script lang="ts">
  import { Pencil, Plus, Trash2 } from 'lucide-svelte';
  import type { SavedViewSummary } from '../lib/types';

  let {
    views = [],
    activeViewId = null,
    disabled = false,
    onSelect,
    onCreate,
    onRename,
    onDelete,
    onDuplicate,
  }: {
    views: SavedViewSummary[];
    activeViewId: number | null;
    disabled?: boolean;
    onSelect: (view: SavedViewSummary) => void;
    onCreate: () => void;
    onRename: (view: SavedViewSummary, name: string) => void;
    onDelete: (view: SavedViewSummary) => void;
    onDuplicate: (view: SavedViewSummary) => void;
  } = $props();

  let search = $state('');

  const filteredViews = $derived.by(() => {
    const query = search.trim().toLowerCase();
    if (!query) return views;
    return views.filter((view) => view.name.toLowerCase().includes(query));
  });
  function handleRename(view: SavedViewSummary) {
    const nextName = window.prompt('Rename saved view', view.name)?.trim();
    if (!nextName || nextName === view.name) return;
    onRename(view, nextName);
  }

  function handleDelete(view: SavedViewSummary) {
    if (!window.confirm(`Delete saved view "${view.name}"?`)) return;
    onDelete(view);
  }
</script>

<section class="view-list">
  <div class="section-header">
    <div>
      <h3>Views</h3>
      <p>Saved joined and aggregated views</p>
    </div>
    <button type="button" class="create-btn" onclick={onCreate} disabled={disabled}>
      <Plus size={14} />
      <span>Create</span>
    </button>
  </div>

  <input
    bind:value={search}
    class="view-search"
    type="search"
    placeholder="Search views"
    aria-label="Search saved views"
    spellcheck="false"
  />

  {#if filteredViews.length > 0}
    <div class="items">
      {#each filteredViews as view (view.id)}
        <div class="view-row" class:active={activeViewId === view.id}>
          <button type="button" class="view-item" onclick={() => onSelect(view)} disabled={disabled}>
            <span class="view-item-name">{view.name}</span>
            <span class="view-item-meta">{view.base_schema}.{view.base_table}</span>
          </button>
          <div class="view-actions">
            <button type="button" class="icon-btn" aria-label={`Duplicate ${view.name}`} onclick={() => onDuplicate(view)} disabled={disabled}>
              <Plus size={14} />
            </button>
            <button type="button" class="icon-btn" aria-label={`Rename ${view.name}`} onclick={() => handleRename(view)} disabled={disabled}>
              <Pencil size={14} />
            </button>
            <button type="button" class="icon-btn" aria-label={`Delete ${view.name}`} onclick={() => handleDelete(view)} disabled={disabled}>
              <Trash2 size={14} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state">No saved views match “{search.trim()}”.</div>
  {/if}
</section>

<style>
  .view-list {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md) var(--sk-space-md);
    border-top: 1px solid var(--sk-border-light);
  }

  .section-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-md);
  }

  .section-header h3,
  .section-header p {
    margin: 0;
  }

  .section-header h3 {
    font-size: var(--sk-font-size-sm);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--sk-secondary-strong);
  }

  .section-header p {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
  }

  .create-btn,
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: var(--sk-radius-md);
    font: inherit;
    cursor: pointer;
  }

  .create-btn {
    border: 1px solid rgba(0, 169, 165, 0.24);
    background: rgba(0, 169, 165, 0.08);
    color: var(--sk-accent);
    padding: 6px 10px;
  }

  .view-search {
    width: 100%;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.72);
    color: var(--sk-text);
    padding: 6px 10px;
    font: inherit;
  }

  .items {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .view-row {
    display: flex;
    align-items: stretch;
    gap: 6px;
    border-radius: var(--sk-radius-sm);
  }

  .view-row.active {
    background: rgba(0, 169, 165, 0.1);
  }

  .view-item {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    border: none;
    background: transparent;
    color: var(--sk-text);
    text-align: left;
    padding: var(--sk-space-xs);
    border-radius: var(--sk-radius-sm);
    cursor: pointer;
  }

  .view-item:hover,
  .view-row.active .view-item {
    background: rgba(0, 169, 165, 0.08);
  }

  .view-item-name {
    font-size: var(--sk-font-size-body);
  }

  .view-item-meta {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
  }

  .view-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .icon-btn {
    width: 30px;
    height: 30px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--sk-secondary-strong);
  }

  .icon-btn:hover {
    border-color: var(--sk-border-light);
    background: rgba(255, 255, 255, 0.7);
  }

  .create-btn:disabled,
  .view-item:disabled,
  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .empty-state {
    color: var(--sk-muted);
    font-size: var(--sk-font-size-body);
  }
</style>
