<script lang="ts">
  import { CheckCircle, AlertCircle, Loader, Info } from 'lucide-svelte';
  import { setupTestConnection } from '../lib/api';
  import type { WizardData, TestConnectionResult } from '../lib/types';

  let {
    wizardData = $bindable(),
    onNext,
  }: {
    wizardData: WizardData;
    onNext: () => void;
  } = $props();

  type TestState = 'idle' | 'loading' | 'success' | 'error';
  let testState: TestState = $state('idle');
  let testResult: TestConnectionResult | null = $state(null);
  let testError: string = $state('');

  // Build postgres URL from fields
  let fieldUrl = $derived(
    wizardData.connection_mode === 'fields'
      ? `postgresql://${wizardData.db_user}:${wizardData.db_password ? '••••••' : ''}@${wizardData.host}:${wizardData.port}/${wizardData.database}`
      : ''
  );

  function buildActualUrl(): string {
    if (wizardData.connection_mode === 'url') return wizardData.url;
    const user = encodeURIComponent(wizardData.db_user);
    const pass = encodeURIComponent(wizardData.db_password);
    return `postgresql://${user}:${pass}@${wizardData.host}:${wizardData.port}/${wizardData.database}`;
  }

  async function runTest() {
    testState = 'loading';
    testResult = null;
    testError = '';
    try {
      const req: { kind: string; url: string; ssh?: typeof wizardData.ssh } = {
        kind: 'postgres',
        url: buildActualUrl(),
      };
      if (wizardData.use_ssh) {
        req.ssh = wizardData.ssh;
      }
      const result = await setupTestConnection(req);
      testResult = result;
      if (result.success) {
        wizardData.tables = result.tables ?? [];
        wizardData.schemas = result.schemas ?? [];
        // Default schema selection: public if available, else everything.
        const names = wizardData.schemas.map((s) => s.name);
        if (names.includes('public')) {
          wizardData.selected_schemas = ['public'];
        } else {
          wizardData.selected_schemas = names;
        }
        testState = 'success';
      } else {
        const prefix = result.error_source === 'ssh' ? 'SSH: ' : result.error_source === 'db' ? 'Database: ' : '';
        testError = prefix + (result.error ?? 'Connection failed');
        testState = 'error';
      }
    } catch (e) {
      testError = e instanceof Error ? e.message : String(e);
      testState = 'error';
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && wizardData.connection_mode === 'url') {
      wizardData.url = '';
    }
    if (e.key === 'Enter' && testState === 'success') {
      onNext();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="step" role="form" onkeydown={handleKeydown}>

  <!-- Connection mode toggle -->
  <div class="mode-tabs">
    <button
      class="mode-tab"
      class:active={wizardData.connection_mode === 'url'}
      onclick={() => { wizardData.connection_mode = 'url'; testState = 'idle'; }}
      aria-pressed={wizardData.connection_mode === 'url'}
    >
      Connection URL
    </button>
    <button
      class="mode-tab"
      class:active={wizardData.connection_mode === 'fields'}
      onclick={() => { wizardData.connection_mode = 'fields'; testState = 'idle'; }}
      aria-pressed={wizardData.connection_mode === 'fields'}
    >
      Fill in fields
    </button>
  </div>

  {#if wizardData.connection_mode === 'url'}
    <div class="field">
      <label for="conn-url">Connection URL</label>
      <input
        id="conn-url"
        type="text"
        placeholder="postgresql://user:pass@localhost:5432/mydb"
        bind:value={wizardData.url}
        oninput={() => testState = 'idle'}
        autocomplete="off"
        spellcheck="false"
      />
      <p class="hint">Press <kbd>Esc</kbd> to clear</p>
    </div>
  {:else}
    <div class="fields-grid">
      <div class="field span-2">
        <label for="db-host">Host</label>
        <input id="db-host" type="text" bind:value={wizardData.host} oninput={() => testState = 'idle'} />
      </div>
      <div class="field">
        <label for="db-port">Port</label>
        <input id="db-port" type="number" min="1" max="65535" bind:value={wizardData.port} oninput={() => testState = 'idle'} />
      </div>
      <div class="field span-2">
        <label for="db-name">Database</label>
        <input id="db-name" type="text" bind:value={wizardData.database} oninput={() => testState = 'idle'} />
      </div>
      <div class="field">
        <label for="db-user">User</label>
        <input id="db-user" type="text" bind:value={wizardData.db_user} oninput={() => testState = 'idle'} autocomplete="username" />
      </div>
      <div class="field">
        <label for="db-pass">Password</label>
        <input id="db-pass" type="password" bind:value={wizardData.db_password} oninput={() => testState = 'idle'} autocomplete="current-password" />
      </div>
    </div>
    {#if wizardData.db_user || wizardData.host}
      <div class="url-preview">
        <span class="url-preview-label">Preview:</span>
        <code>{fieldUrl}</code>
      </div>
    {/if}
  {/if}

  <!-- SSH Tunnel section -->
  <div class="ssh-section">
    <label class="toggle-row">
      <input
        type="checkbox"
        bind:checked={wizardData.use_ssh}
        onchange={() => testState = 'idle'}
      />
      <span class="toggle-label">Connect via SSH Tunnel</span>
    </label>

    {#if wizardData.use_ssh}
      <div class="ssh-fields">
        <div class="fields-grid">
          <div class="field span-2">
            <label for="ssh-host">SSH Host</label>
            <input id="ssh-host" type="text" bind:value={wizardData.ssh.host} oninput={() => testState = 'idle'} />
          </div>
          <div class="field">
            <label for="ssh-port">SSH Port</label>
            <input id="ssh-port" type="number" min="1" max="65535" bind:value={wizardData.ssh.port} oninput={() => testState = 'idle'} />
          </div>
          <div class="field span-2">
            <label for="ssh-user">SSH Username</label>
            <input id="ssh-user" type="text" bind:value={wizardData.ssh.username} oninput={() => testState = 'idle'} autocomplete="username" />
          </div>
          <div class="field span-3">
            <label for="ssh-auth">Auth Method</label>
            <select id="ssh-auth" bind:value={wizardData.ssh.auth_method} onchange={() => testState = 'idle'}>
              <option value="key">Private Key File</option>
              <option value="agent">SSH Agent</option>
              <option value="password">Password (limited)</option>
            </select>
          </div>
        </div>

        {#if wizardData.ssh.auth_method === 'key'}
          <div class="fields-grid">
            <div class="field span-3">
              <label for="ssh-key-path">Key Path</label>
              <input id="ssh-key-path" type="text" placeholder="~/.ssh/id_rsa" bind:value={wizardData.ssh.key_path} oninput={() => testState = 'idle'} />
            </div>
            <div class="field span-3">
              <label for="ssh-passphrase">Passphrase <span class="optional">(optional)</span></label>
              <input id="ssh-passphrase" type="password" bind:value={wizardData.ssh.key_passphrase} oninput={() => testState = 'idle'} autocomplete="off" />
            </div>
          </div>
        {:else if wizardData.ssh.auth_method === 'password'}
          <div class="fields-grid">
            <div class="field span-3">
              <label for="ssh-pw">SSH Password</label>
              <input id="ssh-pw" type="password" bind:value={wizardData.ssh.password} oninput={() => testState = 'idle'} autocomplete="current-password" />
            </div>
          </div>
          <div class="info-banner warning">
            <AlertCircle size={14} />
            <span>Password-based SSH auth may not be supported on all servers.</span>
          </div>
        {:else if wizardData.ssh.auth_method === 'agent'}
          <div class="info-banner info">
            <Info size={14} />
            <span>Will use <code>SSH_AUTH_SOCK</code> agent socket.</span>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Test connection -->
  <div class="test-row">
    <button
      class="btn-test"
      onclick={runTest}
      disabled={testState === 'loading'}
      aria-label="Test database connection"
    >
      {#if testState === 'loading'}
        <Loader size={14} class="spin" />
        Testing…
      {:else}
        Test Connection
      {/if}
    </button>

    {#if testState === 'success'}
      <span class="test-ok">
        <CheckCircle size={14} />
        Connected — {testResult?.tables?.length ?? 0} tables found
      </span>
    {:else if testState === 'error'}
      <span class="test-err">
        <AlertCircle size={14} />
        {testError}
      </span>
    {/if}
  </div>

  <!-- Next -->
  <div class="actions">
    <button
      class="btn-next"
      onclick={onNext}
      disabled={testState !== 'success'}
      aria-label="Proceed to table selection"
    >
      Next →
    </button>
    {#if testState !== 'success'}
      <span class="next-hint">Test your connection first</span>
    {/if}
  </div>
</div>

<style>
  .step { display: flex; flex-direction: column; gap: var(--sk-space-lg); }

  .mode-tabs {
    display: flex;
    gap: var(--sk-space-xs);
    background: var(--sk-border);
    border-radius: var(--sk-radius-md);
    padding: 3px;
  }
  .mode-tab {
    flex: 1;
    padding: var(--sk-space-xs) var(--sk-space-sm);
    border: none;
    background: transparent;
    border-radius: calc(var(--sk-radius-md) - 2px);
    font-size: var(--sk-font-size-body);
    color: var(--sk-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .mode-tab.active {
    background: white;
    color: var(--sk-text);
    font-weight: 500;
    box-shadow: 0 1px 4px rgba(47,72,88,0.08);
  }

  .field { display: flex; flex-direction: column; gap: 4px; }
  .field label {
    font-size: var(--sk-font-size-body);
    font-weight: 500;
    color: var(--sk-text);
  }
  .field input, .field select {
    background: var(--sk-glass-input);
    border: 1px solid rgba(47,72,88,0.14);
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
    font-family: var(--sk-font-ui);
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .field input:focus, .field select:focus {
    border-color: var(--sk-accent);
    box-shadow: 0 0 0 3px rgba(0,169,165,0.12);
  }
  .hint { font-size: var(--sk-font-size-sm); color: var(--sk-muted); margin: 0; }
  kbd {
    font-family: var(--sk-font-mono);
    background: rgba(47,72,88,0.07);
    border: 1px solid rgba(47,72,88,0.12);
    border-radius: 3px;
    padding: 0 4px;
    font-size: 10px;
  }

  .fields-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: var(--sk-space-md);
  }
  .span-2 { grid-column: span 2; }
  .span-3 { grid-column: span 3; }

  .url-preview {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    background: rgba(0,169,165,0.06);
    border: 1px solid rgba(0,169,165,0.15);
    border-radius: var(--sk-radius-md);
    padding: var(--sk-space-sm) var(--sk-space-md);
    font-size: var(--sk-font-size-body);
  }
  .url-preview-label { color: var(--sk-muted); white-space: nowrap; }
  .url-preview code {
    font-family: var(--sk-font-mono);
    font-size: 11px;
    color: var(--sk-secondary-strong);
    word-break: break-all;
  }

  .ssh-section {
    border: 1px solid var(--sk-border);
    border-radius: var(--sk-radius-lg);
    padding: var(--sk-space-md);
    background: rgba(255,255,255,0.4);
  }
  .toggle-row {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    cursor: pointer;
    user-select: none;
  }
  .toggle-row input[type=checkbox] {
    width: 16px; height: 16px; accent-color: var(--sk-accent); cursor: pointer;
  }
  .toggle-label { font-size: var(--sk-font-size-body); font-weight: 500; color: var(--sk-text); }
  .ssh-fields { margin-top: var(--sk-space-md); display: flex; flex-direction: column; gap: var(--sk-space-md); }

  .info-banner {
    display: flex;
    align-items: flex-start;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-body);
  }
  .info-banner.warning {
    background: rgba(245,158,11,0.08);
    border: 1px solid rgba(245,158,11,0.2);
    color: #92400e;
  }
  .info-banner.info {
    background: rgba(0,169,165,0.07);
    border: 1px solid rgba(0,169,165,0.15);
    color: var(--sk-secondary-strong);
  }
  .info-banner code { font-family: var(--sk-font-mono); font-size: 11px; }
  .optional { font-weight: 400; color: var(--sk-muted); }

  .test-row {
    display: flex;
    align-items: center;
    gap: var(--sk-space-md);
    flex-wrap: wrap;
  }
  .btn-test {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-sm) var(--sk-space-lg);
    background: var(--sk-glass-button);
    border: 1px solid rgba(47,72,88,0.14);
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-body);
    font-weight: 500;
    color: var(--sk-text);
    cursor: pointer;
    transition: background 0.15s, box-shadow 0.15s;
  }
  .btn-test:hover:not(:disabled) { background: white; box-shadow: var(--sk-shadow-card); }
  .btn-test:disabled { opacity: 0.6; cursor: not-allowed; }

  .test-ok {
    display: inline-flex; align-items: center; gap: 5px;
    color: #15803d; font-size: var(--sk-font-size-body); font-weight: 500;
  }
  .test-err {
    display: inline-flex; align-items: flex-start; gap: 5px;
    color: #b91c1c; font-size: var(--sk-font-size-body);
    flex: 1; min-width: 0;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--sk-space-md);
    margin-top: var(--sk-space-xs);
  }
  .btn-next {
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
  .btn-next:hover:not(:disabled) { opacity: 0.9; box-shadow: 0 4px 12px rgba(0,169,165,0.3); }
  .btn-next:disabled { opacity: 0.45; cursor: not-allowed; box-shadow: none; }
  .next-hint { font-size: var(--sk-font-size-sm); color: var(--sk-muted); }

  :global(.spin) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
