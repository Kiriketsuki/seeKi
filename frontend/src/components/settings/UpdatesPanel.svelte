<script lang="ts">
  import {
    AlertCircle,
    CircleCheck,
    Download,
    RefreshCw,
    RotateCcw,
    Upload,
    X,
  } from 'lucide-svelte';
  import PanelFrame from './PanelFrame.svelte';
  import {
    applyUpdate,
    checkForUpdates,
    fetchUpdateStatus,
    fetchVersion,
    rollbackUpdate,
    updateSettings,
    uploadWipBinary,
  } from '../../lib/api';
  import type {
    UpdatePollIntervalHours,
    UpdateStatus,
    VersionInfo,
    WipUploadResult,
  } from '../../lib/types';

  const pollIntervals: Array<{ label: string; value: UpdatePollIntervalHours }> = [
    { label: 'Hourly', value: 1 },
    { label: 'Every 6 hours', value: 6 },
    { label: 'Daily', value: 24 },
    { label: 'Never', value: 0 },
  ];

  let {
    initialStatus = null,
    onStatusChange,
  }: {
    initialStatus?: UpdateStatus | null;
    onStatusChange?: (status: UpdateStatus) => void;
  } = $props();

  let status = $state<UpdateStatus | null>(null);
  let versionInfo = $state<VersionInfo | null>(null);
  let wipResult = $state<WipUploadResult | null>(null);
  let checking = $state(false);
  let applying = $state(false);
  let rollingBack = $state(false);
  let uploading = $state(false);
  let restarting = $state(false);
  let savingSettings = $state(false);
  let errorMsg = $state<string | null>(null);
  let successMsg = $state<string | null>(null);
  let confirmAction = $state<'install' | 'rollback' | 'wip' | null>(null);
  let dragOver = $state(false);
  let fileInputEl = $state<HTMLInputElement | null>(null);
  let restartPollId = $state<ReturnType<typeof setInterval> | null>(null);

  $effect(() => {
    if (initialStatus === undefined) return;
    status = initialStatus;
  });

  let updateAvailable = $derived(status?.update_available ?? false);
  let previousExists = $derived(status?.previous_exists ?? false);
  let pollIntervalHours = $derived((status?.poll_interval_hours ?? 6) as UpdatePollIntervalHours);

  let lastCheckedLabel = $derived.by(() => {
    if (!status?.last_checked) return '—';
    const parsed = new Date(status.last_checked);
    return Number.isNaN(parsed.getTime()) ? status.last_checked : parsed.toLocaleString();
  });

  void initialize();

  async function initialize() {
    try {
      const [fetchedVersion, fetchedStatus] = await Promise.all([
        fetchVersion(),
        fetchUpdateStatus(),
      ]);
      versionInfo = fetchedVersion;
      if (fetchedStatus) {
        setStatus(fetchedStatus);
      }
    } catch (error) {
      errorMsg = error instanceof Error ? error.message : 'Failed to load update status';
    }
  }

  function setStatus(nextStatus: UpdateStatus) {
    status = nextStatus;
    onStatusChange?.(nextStatus);
  }

  async function handleCheck() {
    checking = true;
    errorMsg = null;
    successMsg = null;
    try {
      const nextStatus = await checkForUpdates();
      if (nextStatus) {
        setStatus(nextStatus);
      }
      successMsg = nextStatus?.update_available
        ? `Update available: ${nextStatus.latest}`
        : 'You are running the latest version.';
    } catch (error) {
      errorMsg = error instanceof Error ? error.message : 'Failed to check for updates';
    } finally {
      checking = false;
    }
  }

  async function handleInstall() {
    confirmAction = null;
    applying = true;
    errorMsg = null;
    successMsg = null;
    try {
      await applyUpdate('release');
      restarting = true;
      startReconnectPolling();
    } catch (error) {
      errorMsg = error instanceof Error ? error.message : 'Install failed';
      applying = false;
    }
  }

  async function handleRollback() {
    confirmAction = null;
    rollingBack = true;
    errorMsg = null;
    successMsg = null;
    try {
      await rollbackUpdate();
      restarting = true;
      startReconnectPolling();
    } catch (error) {
      errorMsg = error instanceof Error ? error.message : 'Rollback failed';
      rollingBack = false;
    }
  }

  async function handleApplyWip() {
    if (!wipResult) return;
    confirmAction = null;
    applying = true;
    errorMsg = null;
    successMsg = null;
    try {
      await applyUpdate('wip', wipResult.upload_id);
      restarting = true;
      startReconnectPolling();
    } catch (error) {
      errorMsg = error instanceof Error ? error.message : 'WIP apply failed';
      applying = false;
    }
  }

  function startReconnectPolling() {
    if (restartPollId) clearInterval(restartPollId);
    let attempts = 0;
    restartPollId = setInterval(async () => {
      attempts += 1;
      try {
        await fetchVersion();
        if (restartPollId) clearInterval(restartPollId);
        restartPollId = null;
        window.location.reload();
      } catch {
        if (attempts < 30) return;
        if (restartPollId) clearInterval(restartPollId);
        restartPollId = null;
        restarting = false;
        applying = false;
        rollingBack = false;
        errorMsg = 'Server did not come back online. Please check manually.';
      }
    }, 2000);
  }

  async function handleChannelToggle() {
    if (!status) return;
    savingSettings = true;
    errorMsg = null;
    try {
      const nextStatus = await updateSettings({
        preReleaseChannel: !status.pre_release_channel,
      });
      setStatus(nextStatus);
    } catch (error) {
      errorMsg = error instanceof Error ? error.message : 'Failed to update settings';
    } finally {
      savingSettings = false;
    }
  }

  async function handlePollIntervalChange(event: Event) {
    savingSettings = true;
    errorMsg = null;
    try {
      const value = Number((event.currentTarget as HTMLSelectElement).value) as UpdatePollIntervalHours;
      const nextStatus = await updateSettings({ pollIntervalHours: value });
      setStatus(nextStatus);
    } catch (error) {
      errorMsg = error instanceof Error ? error.message : 'Failed to update settings';
    } finally {
      savingSettings = false;
    }
  }

  function handleFileDrop(event: DragEvent) {
    event.preventDefault();
    dragOver = false;
    const file = event.dataTransfer?.files?.[0];
    if (file) {
      void uploadFile(file);
    }
  }

  function handleFileSelect(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (file) {
      void uploadFile(file);
    }
    (event.currentTarget as HTMLInputElement).value = '';
  }

  async function uploadFile(file: File) {
    uploading = true;
    errorMsg = null;
    successMsg = null;
    try {
      wipResult = await uploadWipBinary(file);
      successMsg = 'Binary uploaded successfully.';
    } catch (error) {
      errorMsg = error instanceof Error ? error.message : 'Upload failed';
    } finally {
      uploading = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<PanelFrame
  title="Updates"
  description="Check for releases, choose your update channel, install a published build, upload a WIP binary, or roll back to the previous executable."
>
  {#if restarting}
    <div class="restart-card">
      <div class="loading-spinner"></div>
      <p class="restart-title">Restarting SeeKi…</p>
      <p class="restart-copy">Waiting for the updated server to come back online.</p>
    </div>
  {:else}
    {#if errorMsg}
      <div class="banner banner-error">
        <AlertCircle size={14} />
        <span>{errorMsg}</span>
        <button class="banner-dismiss" type="button" onclick={() => (errorMsg = null)} aria-label="Dismiss error">
          <X size={12} />
        </button>
      </div>
    {/if}

    {#if successMsg}
      <div class="banner banner-success">
        <CircleCheck size={14} />
        <span>{successMsg}</span>
        <button class="banner-dismiss" type="button" onclick={() => (successMsg = null)} aria-label="Dismiss success message">
          <X size={12} />
        </button>
      </div>
    {/if}

    {#if confirmAction}
      <div class="confirm-card">
        <p class="confirm-copy">
          {#if confirmAction === 'install'}
            Install update {status?.latest ?? ''}? SeeKi will restart after the binary swap.
          {:else if confirmAction === 'rollback'}
            Roll back to the previous binary? SeeKi will restart immediately afterwards.
          {:else}
            Apply the uploaded WIP binary ({wipResult ? formatBytes(wipResult.size) : ''})? SeeKi will restart after the swap.
          {/if}
        </p>
        <div class="confirm-actions">
          <button class="btn btn-secondary" type="button" onclick={() => (confirmAction = null)}>Cancel</button>
          <button
            class="btn btn-accent"
            type="button"
            onclick={() => {
              if (confirmAction === 'install') void handleInstall();
              else if (confirmAction === 'rollback') void handleRollback();
              else void handleApplyWip();
            }}
          >
            Confirm
          </button>
        </div>
      </div>
    {/if}

    <div class="status-grid">
      <div class="status-card">
        <span>Current version</span>
        <strong>{versionInfo?.version ?? status?.current ?? '—'}</strong>
      </div>
      <div class="status-card">
        <span>Latest version</span>
        <strong>{status?.latest ?? 'No release found'}</strong>
      </div>
      <div class="status-card">
        <span>Last checked</span>
        <strong>{lastCheckedLabel}</strong>
      </div>
      <div class="status-card">
        <span>Rollback</span>
        <strong>{previousExists ? 'Available' : 'Unavailable'}</strong>
      </div>
    </div>

    <div class="status-pill-row">
      {#if updateAvailable}
        <span class="status-pill status-pill--warning">
          <AlertCircle size={12} />
          Update available
        </span>
      {:else}
        <span class="status-pill status-pill--ok">
          <CircleCheck size={12} />
          Up to date
        </span>
      {/if}
      {#if versionInfo?.commit}
        <span class="build-meta">Build {versionInfo.commit.slice(0, 7)}</span>
      {/if}
    </div>

    <div class="settings-grid">
      <label class="setting-card">
        <span class="setting-title">Poll interval</span>
        <span class="setting-copy">Choose how often the open app checks for new releases.</span>
        <select value={pollIntervalHours} onchange={handlePollIntervalChange} disabled={savingSettings}>
          {#each pollIntervals as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>

      <div class="setting-card">
        <span class="setting-title">Pre-release channel</span>
        <span class="setting-copy">Enable prerelease builds from work branches and manual dispatches.</span>
        <button
          class="toggle"
          class:toggle--active={status?.pre_release_channel ?? false}
          type="button"
          onclick={handleChannelToggle}
          disabled={savingSettings}
          role="switch"
          aria-label="Toggle pre-release channel"
          aria-checked={status?.pre_release_channel ?? false}
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>
    </div>

    <div class="actions">
      <button class="btn btn-secondary" type="button" onclick={handleCheck} disabled={checking}>
        <span class:spin={checking}>
          <RefreshCw size={14} />
        </span>
        {checking ? 'Checking…' : 'Check for updates'}
      </button>
      <button class="btn btn-accent" type="button" onclick={() => (confirmAction = 'install')} disabled={!updateAvailable || applying}>
        <Download size={14} />
        {applying ? 'Installing…' : 'Install update'}
      </button>
      <button class="btn btn-secondary" type="button" onclick={() => (confirmAction = 'rollback')} disabled={!previousExists || rollingBack}>
        <RotateCcw size={14} />
        {rollingBack ? 'Rolling back…' : 'Rollback'}
      </button>
    </div>

    <section class="section">
      <h2>Release notes</h2>
      {#if status?.release_notes}
        <pre>{status.release_notes}</pre>
      {:else}
        <p class="muted">Run a check or wait for the background poller to cache release notes.</p>
      {/if}
    </section>

    <section class="section">
      <h2>WIP build upload</h2>
      <p class="muted">Upload a local Linux x86_64 ELF build for one-off testing.</p>
      <div
        class="drop-zone"
        class:drop-zone--active={dragOver}
        role="button"
        tabindex="0"
        ondrop={handleFileDrop}
        ondragover={(event) => {
          event.preventDefault();
          dragOver = true;
        }}
        ondragleave={() => (dragOver = false)}
        onclick={() => fileInputEl?.click()}
        onkeydown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            fileInputEl?.click();
          }
        }}
      >
        {#if uploading}
          <div class="loading-spinner small"></div>
          <span>Uploading…</span>
        {:else}
          <Upload size={18} />
          <span>Drop a binary here or click to browse</span>
        {/if}
      </div>
      <input bind:this={fileInputEl} type="file" class="hidden-input" onchange={handleFileSelect} />

      {#if wipResult}
        <div class="wip-card">
          <div class="wip-meta">
            <span>SHA-256</span>
            <strong>{wipResult.sha256.slice(0, 16)}…</strong>
          </div>
          <div class="wip-meta">
            <span>Size</span>
            <strong>{formatBytes(wipResult.size)}</strong>
          </div>
          <button class="btn btn-accent" type="button" onclick={() => (confirmAction = 'wip')} disabled={applying}>
            <Download size={14} />
            Apply WIP
          </button>
        </div>
      {/if}
    </section>
  {/if}
</PanelFrame>

<style>
  .restart-card,
  .section,
  .confirm-card,
  .setting-card,
  .status-card,
  .wip-card {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: rgba(255, 255, 255, 0.7);
    box-shadow: var(--sk-shadow-card);
  }

  .restart-card {
    min-height: 320px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-md);
  }

  .restart-title {
    margin: 0;
    font-size: var(--sk-font-size-lg);
    font-weight: 600;
    color: var(--sk-text);
  }

  .restart-copy,
  .muted {
    color: var(--sk-muted);
  }

  .banner {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    border-radius: var(--sk-radius-md);
  }

  .banner span {
    flex: 1;
  }

  .banner-error {
    border: 1px solid rgba(220, 38, 38, 0.18);
    background: rgba(220, 38, 38, 0.08);
    color: #b91c1c;
  }

  .banner-success {
    border: 1px solid rgba(21, 128, 61, 0.18);
    background: rgba(21, 128, 61, 0.08);
    color: #15803d;
  }

  .banner-dismiss {
    border: none;
    background: none;
    color: inherit;
    cursor: pointer;
  }

  .confirm-card,
  .section,
  .setting-card,
  .wip-card {
    padding: var(--sk-space-lg);
  }

  .confirm-copy {
    margin: 0 0 var(--sk-space-md);
    color: var(--sk-text);
  }

  .confirm-actions,
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-sm);
  }

  .status-grid,
  .settings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--sk-space-md);
  }

  .status-card,
  .setting-card {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
  }

  .status-card span,
  .setting-copy {
    color: var(--sk-muted);
  }

  .status-card strong {
    color: var(--sk-text);
  }

  .status-pill-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: 4px var(--sk-space-sm);
    border-radius: 999px;
    font-size: var(--sk-font-size-sm);
    font-weight: 500;
  }

  .status-pill--ok {
    background: rgba(21, 128, 61, 0.1);
    color: #15803d;
  }

  .status-pill--warning {
    background: rgba(217, 119, 6, 0.12);
    color: #b45309;
  }

  .build-meta {
    color: var(--sk-muted);
    font-family: var(--sk-font-mono);
    font-size: var(--sk-font-size-sm);
  }

  select {
    min-height: 40px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
    background: rgba(255, 255, 255, 0.9);
    color: var(--sk-text);
    padding: 0 var(--sk-space-md);
    font: inherit;
  }

  .toggle {
    width: 52px;
    height: 30px;
    border: none;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.4);
    padding: 4px;
    cursor: pointer;
    transition: background 120ms ease;
  }

  .toggle--active {
    background: var(--sk-accent);
  }

  .toggle-thumb {
    display: block;
    width: 22px;
    height: 22px;
    border-radius: 999px;
    background: white;
    transform: translateX(0);
    transition: transform 120ms ease;
  }

  .toggle--active .toggle-thumb {
    transform: translateX(22px);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    min-height: 40px;
    border-radius: var(--sk-radius-md);
    padding: 0 var(--sk-space-md);
    font: inherit;
    cursor: pointer;
  }

  .btn-secondary {
    border: 1px solid var(--sk-border-light);
    background: var(--sk-glass-button);
    color: var(--sk-text);
  }

  .btn-accent {
    border: none;
    background: var(--sk-accent);
    color: white;
  }

  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
  }

  .section h2 {
    margin: 0;
    color: var(--sk-text);
    font-size: var(--sk-font-size-md);
  }

  pre {
    margin: 0;
    white-space: pre-wrap;
    font-family: var(--sk-font-ui);
    color: var(--sk-text);
    line-height: 1.6;
  }

  .drop-zone {
    min-height: 120px;
    border: 1px dashed var(--sk-border);
    border-radius: var(--sk-radius-lg);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-sm);
    color: var(--sk-muted);
    cursor: pointer;
    background: rgba(255, 255, 255, 0.45);
  }

  .drop-zone--active {
    border-color: var(--sk-accent);
    background: rgba(73, 151, 208, 0.08);
    color: var(--sk-text);
  }

  .hidden-input {
    display: none;
  }

  .wip-card {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sk-space-md);
  }

  .wip-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .wip-meta span {
    color: var(--sk-muted);
  }

  .wip-meta strong {
    color: var(--sk-text);
    font-family: var(--sk-font-mono);
    font-size: var(--sk-font-size-sm);
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(73, 151, 208, 0.2);
    border-top-color: var(--sk-accent);
    border-radius: 999px;
    animation: spin 0.9s linear infinite;
  }

  .loading-spinner.small {
    width: 18px;
    height: 18px;
    border-width: 2px;
  }

  .spin {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
