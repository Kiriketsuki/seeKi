<script lang="ts">
  import { ChevronDown, ChevronUp, GripHorizontal, Plus } from 'lucide-svelte';
  import TableList from './TableList.svelte';
  import ViewList from './ViewList.svelte';
  import {
    createDataPanelLayoutStore,
    type DataPanelId,
  } from '../lib/stores';
  import type { SavedViewSummary, TableInfo } from '../lib/types';

  let {
    tables = [],
    selectedSchema = '',
    selectedTable = '',
    onSelectTable,
    views = [],
    activeViewId = null,
    viewsDisabled = false,
    onSelectView,
    onCreateView,
    onRenameView,
    onDeleteView,
    onDuplicateView,
  }: {
    tables: TableInfo[];
    selectedSchema: string;
    selectedTable: string;
    onSelectTable: (table: TableInfo) => void;
    views: SavedViewSummary[];
    activeViewId: number | null;
    viewsDisabled?: boolean;
    onSelectView: (view: SavedViewSummary) => void;
    onCreateView: () => void;
    onRenameView: (view: SavedViewSummary, name: string) => void;
    onDeleteView: (view: SavedViewSummary) => void;
    onDuplicateView: (view: SavedViewSummary) => void;
  } = $props();

  const layoutStore = createDataPanelLayoutStore();

  let rootEl = $state<HTMLDivElement | null>(null);
  let draggedPanel = $state<DataPanelId | null>(null);
  let dragOverPanel = $state<DataPanelId | null>(null);
  let resizing = $state(false);

  let orderedPanels = $derived.by(() =>
    $layoutStore.order.map((panelId) => ({
      id: panelId,
      title: panelId === 'tables' ? 'Tables' : 'Views',
      subtitle:
        panelId === 'tables'
          ? `${tables.length} available`
          : `${views.length} saved`,
    })),
  );

  let resizeDisabled = $derived.by(() =>
    $layoutStore.collapsed.tables || $layoutStore.collapsed.views
  );

  let gridTemplateRows = $derived.by(() => {
    const [topPanel, bottomPanel] = $layoutStore.order;
    if ($layoutStore.collapsed[topPanel]) {
      return 'var(--sk-data-panel-collapsed) var(--sk-data-panel-grip) minmax(0, 1fr)';
    }

    if ($layoutStore.collapsed[bottomPanel]) {
      return 'minmax(0, 1fr) var(--sk-data-panel-grip) var(--sk-data-panel-collapsed)';
    }

    return `minmax(180px, ${$layoutStore.sizes[topPanel]}fr) var(--sk-data-panel-grip) minmax(180px, ${$layoutStore.sizes[bottomPanel]}fr)`;
  });

  function handleHeaderDragStart(event: DragEvent, panelId: DataPanelId) {
    draggedPanel = panelId;
    dragOverPanel = null;
    event.dataTransfer?.setData('text/plain', panelId);
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function handleHeaderDragOver(event: DragEvent, panelId: DataPanelId) {
    if (!draggedPanel || draggedPanel === panelId) {
      return;
    }

    event.preventDefault();
    dragOverPanel = panelId;
  }

  function handleHeaderDrop(event: DragEvent, panelId: DataPanelId) {
    event.preventDefault();
    if (draggedPanel && draggedPanel !== panelId) {
      layoutStore.swapOrder();
    }

    draggedPanel = null;
    dragOverPanel = null;
  }

  function handleHeaderDragEnd() {
    draggedPanel = null;
    dragOverPanel = null;
  }

  function handleGripPointerDown(event: PointerEvent) {
    if (resizeDisabled) {
      return;
    }

    event.preventDefault();
    resizing = true;
  }

  function handlePointerMove(event: PointerEvent) {
    if (!resizing || !rootEl || resizeDisabled) {
      return;
    }

    const bounds = rootEl.getBoundingClientRect();
    const relativeY = event.clientY - bounds.top;
    const topSize = (relativeY / bounds.height) * 100;
    layoutStore.setTopSize(topSize);
  }

  function handlePointerUp() {
    resizing = false;
  }
</script>

<svelte:window onpointermove={handlePointerMove} onpointerup={handlePointerUp} />

<div
  bind:this={rootEl}
  class="data-panels"
  class:is-resizing={resizing}
  style:grid-template-rows={gridTemplateRows}
  data-testid="data-panels"
>
  {#each orderedPanels as panel, index (panel.id)}
    <section
      class="data-panel"
      class:is-collapsed={$layoutStore.collapsed[panel.id]}
      class:is-drop-target={dragOverPanel === panel.id}
      data-testid={`data-panel-${panel.id}`}
    >
      <header
        class="data-panel__header"
        role="group"
        aria-label={`${panel.title} panel header`}
        draggable="true"
        ondragstart={(event) => handleHeaderDragStart(event, panel.id)}
        ondragover={(event) => handleHeaderDragOver(event, panel.id)}
        ondrop={(event) => handleHeaderDrop(event, panel.id)}
        ondragend={handleHeaderDragEnd}
        data-testid={`data-panel-header-${panel.id}`}
      >
        <div class="data-panel__heading">
          <span class="data-panel__title">{panel.title}</span>
          <span class="data-panel__subtitle">{panel.subtitle}</span>
        </div>
        <div class="data-panel__actions">
          {#if panel.id === 'views'}
            <button
              type="button"
              class="panel-action panel-action--primary"
              onclick={onCreateView}
              disabled={viewsDisabled}
              data-testid="data-panel-create-view"
            >
              <Plus size={14} />
              <span>Create view</span>
            </button>
          {/if}
          <span class="drag-handle" aria-hidden="true">
            <GripHorizontal size={14} />
          </span>
          <button
            type="button"
            class="panel-action panel-action--icon"
            aria-label={$layoutStore.collapsed[panel.id] ? `Expand ${panel.title}` : `Collapse ${panel.title}`}
            onclick={() => layoutStore.toggleCollapsed(panel.id)}
            data-testid={`data-panel-toggle-${panel.id}`}
          >
            {#if $layoutStore.collapsed[panel.id]}
              <ChevronDown size={14} />
            {:else}
              <ChevronUp size={14} />
            {/if}
          </button>
        </div>
      </header>

      {#if !$layoutStore.collapsed[panel.id]}
        <div class="data-panel__body" data-testid={`data-panel-body-${panel.id}`}>
          {#if panel.id === 'tables'}
            <TableList
              {tables}
              {selectedSchema}
              {selectedTable}
              onSelect={onSelectTable}
              showHeader={false}
            />
          {:else}
            <ViewList
              {views}
              {activeViewId}
              disabled={viewsDisabled}
              onSelect={onSelectView}
              onCreate={onCreateView}
              onRename={onRenameView}
              onDelete={onDeleteView}
              onDuplicate={onDuplicateView}
              showHeader={false}
            />
          {/if}
        </div>
      {/if}
    </section>

    {#if index === 0}
      <div
        class="data-panel__grip"
        class:is-disabled={resizeDisabled}
        onpointerdown={handleGripPointerDown}
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize data panels"
        data-testid="data-panel-grip"
      >
        <span></span>
      </div>
    {/if}
  {/each}
</div>

<style>
  .data-panels {
    --sk-data-panel-collapsed: 72px;
    --sk-data-panel-grip: 12px;

    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-height: 0;
    height: 100%;
  }

  .data-panels.is-resizing {
    cursor: row-resize;
    user-select: none;
  }

  .data-panel {
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.58);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.5);
  }

  .data-panel.is-drop-target {
    border-color: rgba(0, 169, 165, 0.45);
    box-shadow: 0 0 0 1px rgba(0, 169, 165, 0.18);
  }

  .data-panel__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    background: rgba(255, 255, 255, 0.88);
    border-bottom: 1px solid var(--sk-border-light);
    cursor: grab;
  }

  .data-panel__header:active {
    cursor: grabbing;
  }

  .data-panel.is-collapsed .data-panel__header {
    border-bottom: none;
  }

  .data-panel__heading {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .data-panel__title {
    font-size: var(--sk-font-size-sm);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--sk-secondary-strong);
  }

  .data-panel__subtitle {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .data-panel__actions {
    display: flex;
    align-items: center;
    gap: var(--sk-space-xs);
    flex-shrink: 0;
  }

  .panel-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: var(--sk-radius-md);
    border: 1px solid transparent;
    font: inherit;
    cursor: pointer;
  }

  .panel-action--primary {
    padding: 6px 10px;
    border-color: rgba(0, 169, 165, 0.24);
    background: rgba(0, 169, 165, 0.08);
    color: var(--sk-accent);
  }

  .panel-action--icon {
    width: 30px;
    height: 30px;
    background: transparent;
    color: var(--sk-secondary-strong);
  }

  .panel-action--icon:hover {
    border-color: var(--sk-border-light);
    background: rgba(255, 255, 255, 0.78);
  }

  .panel-action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .drag-handle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    color: var(--sk-muted);
  }

  .data-panel__body {
    min-height: 0;
    overflow: auto;
    padding: var(--sk-space-sm) 0;
  }

  .data-panel__grip {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: row-resize;
  }

  .data-panel__grip.is-disabled {
    cursor: default;
  }

  .data-panel__grip span {
    width: 56px;
    height: 4px;
    border-radius: 999px;
    background: rgba(47, 72, 88, 0.12);
  }
</style>
