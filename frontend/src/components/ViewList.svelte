<script lang="ts">
  import { Copy, MoreHorizontal, Pencil, Plus, Search, Trash2 } from 'lucide-svelte';
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
    showHeader = true,
  }: {
    views: SavedViewSummary[];
    activeViewId: number | null;
    disabled?: boolean;
    onSelect: (view: SavedViewSummary) => void;
    onCreate: () => void;
    onRename: (view: SavedViewSummary, name: string) => void;
    onDelete: (view: SavedViewSummary) => void;
    onDuplicate: (view: SavedViewSummary) => void;
    showHeader?: boolean;
  } = $props();

  let search = $state('');
  let openMenuFor = $state<number | null>(null);
  let renameTarget = $state<SavedViewSummary | null>(null);
  let renameValue = $state('');
  let deleteTarget = $state<SavedViewSummary | null>(null);

  const filteredViews = $derived.by(() => {
    const query = search.trim().toLowerCase();
    if (!query) {
      return views;
    }

    return views.filter((view) => view.name.toLowerCase().includes(query));
  });

  function handleRename(view: SavedViewSummary) {
    openMenuFor = null;
    renameValue = view.name;
    renameTarget = view;
  }

  function confirmRename() {
    const target = renameTarget;
    const next = renameValue.trim();
    renameTarget = null;
    if (!target || !next || next === target.name) return;
    onRename(target, next);
  }

  function handleDelete(view: SavedViewSummary) {
    openMenuFor = null;
    deleteTarget = view;
  }

  function confirmDelete() {
    const target = deleteTarget;
    deleteTarget = null;
    if (!target) return;
    onDelete(target);
  }

  function handleWindowClick(event: MouseEvent) {
    if (!(event.target instanceof HTMLElement)) {
      openMenuFor = null;
      return;
    }

    if (!event.target.closest('[data-view-actions-root="true"]')) {
      openMenuFor = null;
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      openMenuFor = null;
    }
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<section class="view-list" class:headerless={!showHeader} data-testid="view-list">
  {#if showHeader}
    <div class="section-header">
      <div>
        <h3>Views</h3>
        <p>Saved joined and aggregated views</p>
      </div>
      <button
        type="button"
        class="create-btn"
        onclick={onCreate}
        disabled={disabled}
        data-testid="view-list-create"
      >
        <Plus size={14} />
        <span>Create</span>
      </button>
    </div>
  {/if}

  <label class="panel-search view-search" data-testid="view-list-search">
    <Search size={14} />
    <input
      bind:value={search}
      class="panel-search-input view-search-input"
      type="search"
      placeholder="Search views"
      aria-label="Search saved views"
      spellcheck="false"
      data-testid="view-search-input"
    />
  </label>

  {#if filteredViews.length > 0}
    <div class="items" data-testid="view-list-items">
      {#each filteredViews as view (view.id)}
        <div class="view-row" class:active={activeViewId === view.id}>
          <button
            type="button"
            class="view-item"
            onclick={() => onSelect(view)}
            disabled={disabled}
            data-testid={`view-item-${view.id}`}
          >
            <span class="view-item-name">{view.name}</span>
            <span class="view-item-meta">{view.base_schema}.{view.base_table}</span>
          </button>
          <div
            class="view-actions"
            data-view-actions-root="true"
            class:menu-open={openMenuFor === view.id}
          >
            <button
              type="button"
              class="icon-btn actions-trigger"
              aria-label={`Open actions for ${view.name}`}
              aria-expanded={openMenuFor === view.id}
              onclick={(event) => {
                event.stopPropagation();
                openMenuFor = openMenuFor === view.id ? null : view.id;
              }}
              disabled={disabled}
              data-testid={`view-actions-trigger-${view.id}`}
            >
              <MoreHorizontal size={14} />
            </button>
            {#if openMenuFor === view.id}
              <div class="actions-menu" role="menu" data-testid={`view-actions-menu-${view.id}`}>
                <button
                  type="button"
                  class="actions-menu-item"
                  role="menuitem"
                  onclick={(event) => {
                    event.stopPropagation();
                    openMenuFor = null;
                    onDuplicate(view);
                  }}
                  disabled={disabled}
                  data-testid={`view-actions-copy-${view.id}`}
                >
                  <Copy size={14} />
                  <span>Copy to edit</span>
                </button>
                <button
                  type="button"
                  class="actions-menu-item"
                  role="menuitem"
                  onclick={(event) => {
                    event.stopPropagation();
                    handleRename(view);
                  }}
                  disabled={disabled}
                  data-testid={`view-actions-rename-${view.id}`}
                >
                  <Pencil size={14} />
                  <span>Rename</span>
                </button>
                <button
                  type="button"
                  class="actions-menu-item actions-menu-item--danger"
                  role="menuitem"
                  onclick={(event) => {
                    event.stopPropagation();
                    handleDelete(view);
                  }}
                  disabled={disabled}
                  data-testid={`view-actions-delete-${view.id}`}
                >
                  <Trash2 size={14} />
                  <span>Delete</span>
                </button>
              </div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state" data-testid="view-list-empty">
      No saved views match "{search.trim()}".
    </div>
  {/if}
</section>

{#if renameTarget}
  <div
    class="dialog-backdrop"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) renameTarget = null; }}
  >
    <div class="dialog-card" role="dialog" aria-modal="true" aria-label="Rename view">
      <p class="dialog-title">Rename saved view</p>
      <input
        class="dialog-input"
        type="text"
        bind:value={renameValue}
        onkeydown={(e) => { if (e.key === 'Enter') confirmRename(); if (e.key === 'Escape') renameTarget = null; }}
        data-testid="rename-dialog-input"
      />
      <div class="dialog-actions">
        <button type="button" class="dialog-btn dialog-btn-secondary" onclick={() => renameTarget = null}>Cancel</button>
        <button type="button" class="dialog-btn dialog-btn-primary" onclick={confirmRename} disabled={!renameValue.trim() || renameValue.trim() === renameTarget.name} data-testid="rename-dialog-confirm">Rename</button>
      </div>
    </div>
  </div>
{/if}

{#if deleteTarget}
  <div
    class="dialog-backdrop"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) deleteTarget = null; }}
  >
    <div class="dialog-card" role="dialog" aria-modal="true" aria-label="Delete view">
      <p class="dialog-title">Delete saved view "{deleteTarget.name}"?</p>
      <p class="dialog-detail">This action cannot be undone.</p>
      <div class="dialog-actions">
        <button type="button" class="dialog-btn dialog-btn-secondary" onclick={() => deleteTarget = null}>Cancel</button>
        <button type="button" class="dialog-btn dialog-btn-danger" onclick={confirmDelete} data-testid="delete-dialog-confirm">Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .view-list {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md) var(--sk-space-md);
  }

  .view-list.headerless {
    padding-top: 0;
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

  .panel-search {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    color: var(--sk-muted);
  }

  .panel-search-input {
    width: 100%;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.72);
    color: var(--sk-text);
    padding: 6px 10px;
    font: inherit;
  }

  .panel-search-input:focus {
    border-color: rgba(0, 169, 165, 0.4);
    box-shadow: 0 0 0 2px rgba(0, 169, 165, 0.12);
    outline: none;
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
    position: relative;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 4px;
  }

  .actions-trigger {
    opacity: 0;
    transform: translateY(2px);
    transition: opacity 120ms ease, transform 120ms ease;
  }

  .view-row:hover .actions-trigger,
  .view-row:focus-within .actions-trigger,
  .view-actions.menu-open .actions-trigger {
    opacity: 1;
    transform: translateY(0);
  }

  .actions-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 10;
    min-width: 168px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.96);
    box-shadow: var(--sk-shadow-card);
  }

  .create-btn,
  .icon-btn,
  .actions-menu-item {
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

  .actions-menu-item {
    justify-content: flex-start;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--sk-text);
    padding: 7px 8px;
  }

  .actions-menu-item:hover {
    background: rgba(0, 169, 165, 0.08);
  }

  .actions-menu-item--danger {
    color: #b54747;
  }

  .create-btn:disabled,
  .view-item:disabled,
  .icon-btn:disabled,
  .actions-menu-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .empty-state {
    color: var(--sk-muted);
    font-size: var(--sk-font-size-body);
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

  .dialog-input {
    width: 100%;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.72);
    color: var(--sk-text);
    padding: 8px 10px;
    font: inherit;
    margin-bottom: var(--sk-space-md);
  }

  .dialog-input:focus {
    border-color: rgba(0, 169, 165, 0.4);
    box-shadow: 0 0 0 2px rgba(0, 169, 165, 0.12);
    outline: none;
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

  .dialog-btn-danger {
    border: 1px solid rgba(181, 71, 71, 0.3);
    background: rgba(181, 71, 71, 0.08);
    color: #b54747;
  }

  .dialog-btn-danger:hover:not(:disabled) {
    background: rgba(181, 71, 71, 0.16);
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
