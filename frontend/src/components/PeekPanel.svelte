<script lang="ts">
  import { X } from 'lucide-svelte';
  import type { ColumnInfo } from '../lib/types';
  import { formatCellValue, getColumnDisplayName } from '../lib/data-grid';
  import { isSensitiveColumn } from '../lib/sensitive-columns';
  import { peekCanJump, peekPanelState } from '../lib/peek-panel';

  let {
    open = false,
    title = '',
    loading = false,
    error = null,
    row = null,
    multiple = false,
    columns = [],
    onClose,
    onJump,
    jumpLabel = 'Open',
  }: {
    open?: boolean;
    title?: string;
    loading?: boolean;
    error?: string | null;
    row?: Record<string, unknown> | null;
    multiple?: boolean;
    columns?: ColumnInfo[];
    onClose: () => void;
    onJump?: () => void;
    jumpLabel?: string;
  } = $props();

  let state = $derived(peekPanelState({ loading, error, row }));
  let showJump = $derived(peekCanJump({ loading, error, row }, onJump !== undefined));
</script>

{#if open}
  <div
    class="peek-backdrop"
    role="presentation"
    onclick={onClose}
  >
    <div
      class="peek-panel"
      role="dialog"
      aria-modal="true"
      aria-label={title ? `Preview of ${title}` : 'Linked record preview'}
      tabindex="-1"
      data-testid="fk-peek-panel"
      onclick={(event) => event.stopPropagation()}
    >
      <div class="peek-panel__header">
        <div>
          <p class="peek-panel__eyebrow">Linked record</p>
          <h3>{title}</h3>
        </div>
        <button type="button" class="peek-panel__close" aria-label="Close preview" onclick={onClose}>
          <X size={16} />
        </button>
      </div>

      <div class="peek-panel__body">
        {#if state === 'loading'}
          <div class="peek-panel__state" role="status">
            <div class="peek-panel__spinner"></div>
            <span>Loading…</span>
          </div>
        {:else if state === 'error'}
          <div class="peek-panel__state peek-panel__state--error" role="alert">
            <span>{error}</span>
          </div>
        {:else if state === 'empty'}
          <div class="peek-panel__state" role="status">
            <span>No linked record found</span>
          </div>
        {:else}
          {#if multiple}
            <div class="peek-panel__notice" role="status">
              Several records match. Showing the first.
            </div>
          {/if}
          {@const fields = row ?? {}}
          <dl class="peek-panel__fields">
            {#each columns as column (column.name)}
              {@const formatted = formatCellValue(column, fields[column.name])}
              <div class="peek-panel__field">
                <dt>{getColumnDisplayName(column)}</dt>
                <dd class:is-null={formatted.kind === 'null'}>
                  {#if isSensitiveColumn(column) && formatted.kind !== 'null'}
                    ••••••••
                  {:else}
                    {formatted.display}
                  {/if}
                </dd>
              </div>
            {/each}
          </dl>
        {/if}
      </div>

      {#if showJump && onJump}
        <div class="peek-panel__footer">
          <button type="button" class="peek-panel__jump" onclick={onJump}>
            {jumpLabel}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .peek-backdrop {
    position: fixed;
    inset: 0;
    z-index: 45;
    display: flex;
    justify-content: flex-end;
    background: rgba(var(--sk-ink-rgb), 0.3);
    backdrop-filter: blur(2px);
    -webkit-backdrop-filter: blur(2px);
    animation: sk-fade-in 140ms ease-out;
  }

  .peek-panel {
    width: min(420px, 100%);
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
    overflow-y: auto;
    border-left: 1px solid var(--sk-border-light);
    background: var(--sk-glass-popup);
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    box-shadow: var(--sk-shadow-pop);
    padding: var(--sk-space-xl);
    animation: sk-peek-slide-in 160ms ease-out;
  }

  @keyframes sk-peek-slide-in {
    from {
      transform: translateX(24px);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  .peek-panel__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sk-space-md);
  }

  .peek-panel__eyebrow {
    margin: 0 0 2px;
    font-size: var(--sk-font-size-sm);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--sk-ink-muted);
  }

  .peek-panel__header h3 {
    margin: 0;
    font-size: var(--sk-font-size-lg);
    color: var(--sk-ink);
  }

  .peek-panel__close {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: var(--sk-radius-sm);
    background: transparent;
    color: var(--sk-ink-muted);
    cursor: pointer;
  }

  .peek-panel__close:hover {
    background: rgba(var(--sk-accent-active-rgb), 0.12);
    color: var(--sk-ink);
  }

  .peek-panel__close:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--sk-ring-data);
  }

  .peek-panel__body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-md);
  }

  .peek-panel__state {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    color: var(--sk-ink-muted);
    padding: var(--sk-space-lg) 0;
  }

  .peek-panel__state--error {
    color: var(--sk-danger);
  }

  .peek-panel__spinner {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid rgba(var(--sk-accent-active-rgb), 0.25);
    border-top-color: var(--sk-accent-active);
    animation: sk-spin 0.7s linear infinite;
  }

  @keyframes sk-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .peek-panel__notice {
    padding: var(--sk-space-sm) var(--sk-space-md);
    border-radius: var(--sk-radius-sm);
    background: rgba(var(--sk-accent-count-rgb), 0.14);
    color: var(--sk-ink);
    font-size: var(--sk-font-size-sm);
  }

  .peek-panel__fields {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    margin: 0;
  }

  .peek-panel__field {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-bottom: var(--sk-space-sm);
    border-bottom: 1px solid var(--sk-border-light);
  }

  .peek-panel__field dt {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-ink-muted);
  }

  .peek-panel__field dd {
    margin: 0;
    color: var(--sk-ink);
    word-break: break-word;
  }

  .peek-panel__field dd.is-null {
    color: var(--sk-ink-muted);
    font-style: italic;
  }

  .peek-panel__footer {
    padding-top: var(--sk-space-md);
    border-top: 1px solid var(--sk-border-light);
  }

  .peek-panel__jump {
    width: 100%;
    padding: var(--sk-space-sm) var(--sk-space-md);
    border: none;
    border-radius: var(--sk-radius-sm);
    background: var(--sk-accent);
    color: var(--sk-on-accent);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-body);
    cursor: pointer;
  }

  .peek-panel__jump:hover {
    opacity: 0.9;
  }

  .peek-panel__jump:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--sk-ring-data);
  }
</style>
