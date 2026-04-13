<script lang="ts">
  import { Loader, AlertCircle } from 'lucide-svelte';
  import { setupSaveConfig, getStatus } from '../lib/api';
  import type { WizardData } from '../lib/types';

  let {
    wizardData,
    onBack,
    onGoToStep,
  }: {
    wizardData: WizardData;
    onBack: () => void;
    onGoToStep: (step: number) => void;
  } = $props();

  type SaveState = 'idle' | 'saving' | 'polling' | 'error';
  let saveState: SaveState = $state('idle');
  let saveError: string = $state('');

  function buildUrl(): string {
    if (wizardData.connection_mode === 'url') return wizardData.url;
    const user = encodeURIComponent(wizardData.db_user);
    const pass = encodeURIComponent(wizardData.db_password);
    return `postgresql://${user}:${pass}@${wizardData.host}:${wizardData.port}/${wizardData.database}`;
  }

  // Derive display host for the summary
  let dbHost = $derived(
    wizardData.connection_mode === 'fields'
      ? wizardData.host
      : (() => {
          try {
            return new URL(wizardData.url).hostname;
          } catch {
            return wizardData.url;
          }
        })()
  );

  async function handleSave() {
    saveState = 'saving';
    saveError = '';
    try {
      const result = await setupSaveConfig({
        database: {
          kind: 'postgres',
          url: buildUrl(),
          max_connections: 10,
          schemas: wizardData.selected_schemas.length > 0
            ? wizardData.selected_schemas
            : undefined,
        },
        ssh: wizardData.use_ssh ? wizardData.ssh : undefined,
        tables: wizardData.selected_tables.length > 0
          ? { include: wizardData.selected_tables }
          : undefined,
        branding: {
          title: wizardData.title,
          subtitle: wizardData.subtitle || undefined,
        },
      });

      if (!result.success) {
        saveError = result.error ?? 'Save failed — unknown error';
        saveState = 'error';
        return;
      }

      // Poll /api/status until mode === 'normal'
      saveState = 'polling';
      let attempts = 0;
      const maxAttempts = 20;
      const poll = async () => {
        attempts++;
        try {
          const status = await getStatus();
          if (status.mode === 'normal') {
            window.location.reload();
            return;
          }
        } catch {
          // ignore, keep polling
        }
        if (attempts < maxAttempts) {
          setTimeout(poll, 250);
        } else {
          // Timeout — try reload anyway
          window.location.reload();
        }
      };
      setTimeout(poll, 250);
    } catch (e) {
      saveError = e instanceof Error ? e.message : String(e);
      saveState = 'error';
    }
  }
</script>

<div class="step">
  {#if saveState === 'error'}
    <div class="error-banner" role="alert">
      <AlertCircle size={16} />
      <div class="error-content">
        <span class="error-msg">{saveError}</span>
        <button
          class="btn-go-back"
          onclick={() => onGoToStep(1)}
          aria-label="Go back to connection setup"
        >
          ← Fix connection settings
        </button>
      </div>
    </div>
  {/if}

  <!-- Summary cards -->
  <div class="summary">
    <div class="summary-item">
      <span class="summary-label">Database</span>
      <span class="summary-value">Postgres at <code>{dbHost}</code></span>
    </div>
    <div class="summary-item">
      <span class="summary-label">SSH</span>
      <span class="summary-value">
        {#if wizardData.use_ssh}
          Via SSH tunnel to <code>{wizardData.ssh.host}</code>
        {:else}
          Direct connection
        {/if}
      </span>
    </div>
    <div class="summary-item">
      <span class="summary-label">Schemas</span>
      <span class="summary-value">
        {#if wizardData.selected_schemas.length === 0}
          <em class="muted">(defaults to <code>public</code>)</em>
        {:else}
          {#each wizardData.selected_schemas as schema, i (schema)}
            <code>{schema}</code>{i < wizardData.selected_schemas.length - 1 ? ', ' : ''}
          {/each}
        {/if}
      </span>
    </div>
    <div class="summary-item">
      <span class="summary-label">Tables</span>
      <span class="summary-value">{wizardData.selected_tables.length} tables selected</span>
    </div>
    <div class="summary-item">
      <span class="summary-label">App Title</span>
      <span class="summary-value">
        <strong>{wizardData.title}</strong>
        {#if wizardData.subtitle}
          <span class="subtitle-preview"> — {wizardData.subtitle}</span>
        {/if}
      </span>
    </div>
  </div>

  <!-- Actions -->
  <div class="actions">
    <button
      class="btn-back"
      onclick={onBack}
      disabled={saveState === 'saving' || saveState === 'polling'}
      aria-label="Go back to branding"
    >
      ← Back
    </button>
    <button
      class="btn-save"
      onclick={handleSave}
      disabled={saveState === 'saving' || saveState === 'polling'}
      aria-label="Save configuration and open app"
    >
      {#if saveState === 'saving' || saveState === 'polling'}
        <Loader size={14} class="spin" />
        {saveState === 'polling' ? 'Starting up…' : 'Saving…'}
      {:else}
        Save & Open
      {/if}
    </button>
  </div>
</div>

<style>
  .step { display: flex; flex-direction: column; gap: var(--sk-space-xl); }

  .error-banner {
    display: flex;
    align-items: flex-start;
    gap: var(--sk-space-md);
    background: rgba(220,38,38,0.07);
    border: 1px solid rgba(220,38,38,0.2);
    border-radius: var(--sk-radius-lg);
    padding: var(--sk-space-md) var(--sk-space-lg);
    color: #991b1b;
  }
  .error-content { display: flex; flex-direction: column; gap: var(--sk-space-sm); flex: 1; }
  .error-msg { font-size: var(--sk-font-size-body); line-height: 1.4; }
  .btn-go-back {
    align-self: flex-start;
    padding: var(--sk-space-xs) var(--sk-space-md);
    background: rgba(220,38,38,0.1);
    border: 1px solid rgba(220,38,38,0.2);
    border-radius: var(--sk-radius-sm);
    font-size: var(--sk-font-size-sm);
    color: #991b1b;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn-go-back:hover { background: rgba(220,38,38,0.16); }

  .summary {
    background: rgba(255,255,255,0.5);
    border: 1px solid var(--sk-border);
    border-radius: var(--sk-radius-lg);
    overflow: hidden;
  }
  .summary-item {
    display: flex;
    gap: var(--sk-space-lg);
    align-items: baseline;
    padding: var(--sk-space-md) var(--sk-space-lg);
    border-bottom: 1px solid var(--sk-border);
  }
  .summary-item:last-child { border-bottom: none; }
  .summary-label {
    font-size: var(--sk-font-size-body);
    font-weight: 500;
    color: var(--sk-muted);
    min-width: 80px;
    flex-shrink: 0;
  }
  .summary-value {
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
  }
  .summary-value code {
    font-family: var(--sk-font-mono);
    font-size: 11px;
    background: rgba(47,72,88,0.06);
    padding: 1px 5px;
    border-radius: 3px;
  }
  .subtitle-preview { color: var(--sk-muted); }
  .muted { color: var(--sk-muted); font-style: italic; }

  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .btn-back {
    padding: var(--sk-space-sm) var(--sk-space-lg);
    background: transparent;
    border: 1px solid rgba(47,72,88,0.14);
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .btn-back:hover:not(:disabled) { background: rgba(47,72,88,0.06); color: var(--sk-text); }
  .btn-back:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-save {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-2xl);
    background: var(--sk-accent);
    color: white;
    border: none;
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-md);
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s, box-shadow 0.15s;
    box-shadow: var(--sk-shadow-accent);
  }
  .btn-save:hover:not(:disabled) { opacity: 0.9; box-shadow: 0 4px 12px rgba(0,169,165,0.3); }
  .btn-save:disabled { opacity: 0.6; cursor: not-allowed; box-shadow: none; }

  :global(.spin) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
