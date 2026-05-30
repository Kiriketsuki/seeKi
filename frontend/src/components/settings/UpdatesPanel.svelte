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
  let selectedBuildTag = $state<string | null>(null);

  $effect(() => {
    if (initialStatus === undefined) return;
    status = initialStatus;
  });

  $effect(() => {
    if (!confirmAction) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') confirmAction = null;
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  let updateAvailable = $derived(status?.update_available ?? false);
  let previousExists = $derived(status?.previous_exists ?? false);
  let pollIntervalHours = $derived((status?.poll_interval_hours ?? 6) as UpdatePollIntervalHours);
  let availableBuilds = $derived(status?.available_builds ?? []);

  // Install is enabled if the normal latest-vs-current check flagged an
  // update, OR the user picked a specific build from the dropdown that
  // differs from what is currently running.
  let installEnabled = $derived.by(() => {
    if (applying) return false;
    if (selectedBuildTag && selectedBuildTag !== status?.current) return true;
    return updateAvailable;
  });

  let lastCheckedLabel = $derived.by(() => {
    if (!status?.last_checked) return '—';
    const parsed = new Date(status.last_checked);
    return Number.isNaN(parsed.getTime()) ? status.last_checked : parsed.toLocaleString();
  });

  function formatBuildOption(build: { tag: string; published_at: string }): string {
    const date = new Date(build.published_at);
    const when = Number.isNaN(date.getTime()) ? build.published_at : date.toLocaleString();
    return `${build.tag} — ${when}`;
  }

  // Minimal markdown renderer for release notes. Handles the format emitted by
  // our CI (`## heading`, `- bullet`, `` `code` ``, paragraphs). Escapes HTML
  // before applying inline formatting so untrusted text cannot inject markup.
  function renderReleaseNotes(text: string): string {
    const escape = (s: string) =>
      s
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
    const inline = (s: string) =>
      escape(s).replace(/`([^`]+)`/g, '<code>$1</code>');

    const out: string[] = [];
    let listOpen = false;
    let paraBuf: string[] = [];

    const flushPara = () => {
      if (paraBuf.length > 0) {
        out.push(`<p>${inline(paraBuf.join(' '))}</p>`);
        paraBuf = [];
      }
    };
    const closeList = () => {
      if (listOpen) {
        out.push('</ul>');
        listOpen = false;
      }
    };

    for (const raw of text.split('\n')) {
      const line = raw.trimEnd();

      if (line.trim().length === 0) {
        flushPara();
        closeList();
        continue;
      }

      const heading = /^##\s+(.+)$/.exec(line);
      if (heading) {
        flushPara();
        closeList();
        out.push(`<h3>${inline(heading[1])}</h3>`);
        continue;
      }

      const bullet = /^-\s+(.+)$/.exec(line);
      if (bullet) {
        flushPara();
        if (!listOpen) {
          out.push('<ul>');
          listOpen = true;
        }
        out.push(`<li>${inline(bullet[1])}</li>`);
        continue;
      }

      closeList();
      paraBuf.push(line);
    }

    flushPara();
    closeList();
    return out.join('');
  }

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
      await applyUpdate('release', undefined, selectedBuildTag ?? undefined);
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
      <div
        class="confirm-backdrop"
        role="presentation"
        onclick={(event) => {
          if (event.target === event.currentTarget) confirmAction = null;
        }}
      >
        <div
          class="confirm-card confirm-modal"
          role="dialog"
          aria-modal="true"
          aria-labelledby="confirm-copy"
        >
          <p class="confirm-copy" id="confirm-copy">
            {#if confirmAction === 'install'}
              Install update {selectedBuildTag ?? status?.latest ?? ''}? SeeKi will restart after the binary swap.
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

      {#if status?.pre_release_channel && availableBuilds.length > 0}
        <label class="setting-card">
          <span class="setting-title">Available builds</span>
          <span class="setting-copy">Choose a specific prerelease to install. Defaults to the newest available.</span>
          <select
            value={selectedBuildTag ?? ''}
            onchange={(event) => {
              const value = (event.currentTarget as HTMLSelectElement).value;
              selectedBuildTag = value === '' ? null : value;
            }}
            disabled={applying}
          >
            <option value="">Newest available ({status?.latest ?? '—'})</option>
            {#each availableBuilds as build}
              <option value={build.tag}>{formatBuildOption(build)}</option>
            {/each}
          </select>
        </label>
      {/if}
    </div>

    <div class="actions">
      <button class="btn btn-secondary" type="button" onclick={handleCheck} disabled={checking}>
        <span class:spin={checking}>
          <RefreshCw size={14} />
        </span>
        {checking ? 'Checking…' : 'Check for updates'}
      </button>
      <button class="btn btn-accent" type="button" onclick={() => (confirmAction = 'install')} disabled={!installEnabled}>
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
        <div class="release-notes">{@html renderReleaseNotes(status.release_notes)}</div>
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
  /* ── Shared sk-set-card base ── */
  .restart-card,
  .section,
  .confirm-card,
  .setting-card,
  .status-card,
  .wip-card {
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-lg);
    background: var(--sk-glass-input);
    backdrop-filter: var(--sk-glass-grid-blur);
    -webkit-backdrop-filter: var(--sk-glass-grid-blur);
    box-shadow: var(--sk-shadow-card);
  }

  /* sk-restart-card */
  .restart-card {
    min-height: 300px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: var(--sk-space-md);
    color: var(--sk-accent-active-strong);
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
    margin: 0;
  }

  /* Banners */
  .banner {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-sm);
  }

  .banner span {
    flex: 1;
    min-width: 0;
  }

  .banner-error {
    border: 1px solid rgba(var(--sk-danger-rgb), 0.2);
    background: rgba(var(--sk-danger-rgb), 0.08);
    color: var(--sk-danger);
  }

  .banner-success {
    border: 1px solid rgba(var(--sk-boolean-true-rgb), 0.2);
    background: rgba(var(--sk-boolean-true-rgb), 0.08);
    color: var(--sk-boolean-true);
  }

  .banner-dismiss {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    background: none;
    color: inherit;
    cursor: pointer;
    opacity: 0.6;
    flex-shrink: 0;
  }

  .banner-dismiss:hover {
    opacity: 1;
  }

  /* Confirm backdrop — sk-confirm-backdrop */
  .confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(var(--sk-ink-rgb), 0.4);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sk-space-lg);
    animation: confirm-fade 120ms ease-out;
  }

  /* sk-confirm-modal */
  .confirm-modal {
    max-width: 460px;
    width: 100%;
    background: rgba(255, 255, 255, 0.97);
    box-shadow: var(--sk-shadow-pop);
    animation: confirm-pop 140ms ease-out;
  }

  .confirm-card,
  .section,
  .setting-card,
  .status-card,
  .wip-card {
    padding: var(--sk-space-lg);
  }

  .confirm-copy {
    margin: 0 0 var(--sk-space-md);
    color: var(--sk-text);
    line-height: 1.5;
  }

  /* sk-confirm-actions */
  .confirm-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-sm);
    justify-content: flex-end;
  }

  /* sk-set-actions */
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-sm);
  }

  @keyframes confirm-fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes confirm-pop {
    from { opacity: 0; transform: translateY(8px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  /* sk-settings-grid2 */
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

  /* sk-kv span */
  .status-card span,
  .setting-copy {
    color: var(--sk-muted);
  }

  /* sk-kv strong */
  .status-card strong {
    color: var(--sk-text);
    font-size: var(--sk-font-size-lg);
    overflow-wrap: anywhere;
  }

  .setting-title {
    font-weight: 600;
    color: var(--sk-text);
  }

  /* sk-pill-row */
  .status-pill-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sk-space-sm);
  }

  /* sk-status-pill */
  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xs) var(--sk-space-sm);
    border-radius: var(--sk-radius-pill);
    font-size: var(--sk-font-size-sm);
    font-weight: 500;
  }

  .status-pill--ok {
    background: rgba(var(--sk-boolean-true-rgb), 0.1);
    color: var(--sk-boolean-true);
  }

  /* sk-status-pill.warn — amber */
  .status-pill--warning {
    background: rgba(var(--marble-count-rgb), 0.14);
    color: var(--sk-accent-count-ink);
  }

  /* sk-build-meta */
  .build-meta {
    color: var(--sk-muted);
    font-family: var(--sk-font-mono);
    font-size: var(--sk-font-size-sm);
  }

  select {
    min-height: 38px;
    border: 1px solid var(--sk-border-input);
    border-radius: var(--sk-radius-md);
    background: var(--sk-glass-input);
    color: var(--sk-text);
    padding: 0 var(--sk-space-md);
    font: inherit;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  select:focus {
    border-color: rgba(var(--marble-active-rgb), 0.45);
    box-shadow: 0 0 0 2px var(--sk-ring);
  }

  /* sk-switch — amber ON state */
  .toggle {
    width: 50px;
    height: 28px;
    border: none;
    border-radius: var(--sk-radius-pill);
    background: rgba(var(--marble-vein-rgb), 0.18);
    padding: 3px;
    cursor: pointer;
    transition: background 0.15s ease;
    flex-shrink: 0;
  }

  .toggle--active {
    background: var(--sk-accent);
  }

  .toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* sk-switch-thumb */
  .toggle-thumb {
    display: block;
    width: 22px;
    height: 22px;
    border-radius: var(--sk-radius-pill);
    background: white;
    box-shadow: 0 1px 3px rgba(var(--marble-vein-rgb), 0.2);
    transform: translateX(0);
    transition: transform 0.15s ease;
  }

  .toggle--active .toggle-thumb {
    transform: translateX(22px);
  }

  /* sk-btn-ghost / sk-btn-accent */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-sm) var(--sk-space-md);
    border-radius: var(--sk-radius-md);
    font: inherit;
    font-size: var(--sk-font-size-body);
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease, opacity 0.15s ease;
    white-space: nowrap;
  }

  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
    box-shadow: none;
  }

  /* sk-btn-ghost */
  .btn-secondary {
    background: var(--sk-glass-button);
    backdrop-filter: var(--sk-glass-button-blur);
    -webkit-backdrop-filter: var(--sk-glass-button-blur);
    border-color: var(--sk-border-input);
    color: var(--sk-text);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--sk-glass-button);
    border-color: rgba(var(--marble-active-rgb), 0.24);
    box-shadow: var(--sk-shadow-card);
  }

  /* sk-btn-accent — amber CTA */
  .btn-accent {
    border: none;
    background: var(--sk-accent);
    color: var(--sk-on-accent);
    box-shadow: var(--sk-shadow-accent);
  }

  .btn-accent:hover:not(:disabled) {
    opacity: 0.93;
  }

  /* sk-set-section */
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
  }

  .section h2 {
    margin: 0;
    color: var(--sk-text);
    font-size: var(--sk-font-size-md);
    font-weight: 600;
  }

  /* sk-release-notes */
  .release-notes {
    color: var(--sk-text);
    line-height: 1.6;
  }

  .release-notes :global(h3) {
    margin: var(--sk-space-md) 0 var(--sk-space-sm);
    font-size: var(--sk-font-size-md);
    font-weight: 600;
    color: var(--sk-text);
  }

  .release-notes :global(h3:first-child) {
    margin-top: 0;
  }

  .release-notes :global(p) {
    color: var(--sk-secondary-strong);
    margin: 0 0 var(--sk-space-sm);
  }

  .release-notes :global(p:last-child) {
    margin-bottom: 0;
  }

  .release-notes :global(ul) {
    margin: 0 0 var(--sk-space-sm);
    padding-left: var(--sk-space-lg);
  }

  .release-notes :global(ul:last-child) {
    margin-bottom: 0;
  }

  .release-notes :global(li) {
    color: var(--sk-secondary-strong);
    margin-bottom: var(--sk-space-xs);
  }

  .release-notes :global(li:last-child) {
    margin-bottom: 0;
  }

  .release-notes :global(code) {
    font-family: var(--sk-font-mono);
    font-size: 0.9em;
    background: rgba(var(--marble-vein-rgb), 0.06);
    padding: 0.1em 0.35em;
    border-radius: var(--sk-radius-sm);
  }

  /* sk-dropzone */
  .drop-zone {
    min-height: 120px;
    border: 1px dashed var(--sk-border-input);
    border-radius: var(--sk-radius-lg);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-sm);
    color: var(--sk-muted);
    cursor: pointer;
    background: rgba(255, 255, 255, 0.45);
    transition: border-color 0.15s, background 0.15s, color 0.15s;
  }

  .drop-zone:hover {
    border-color: var(--sk-accent-active);
    background: rgba(var(--marble-active-rgb), 0.05);
    color: var(--sk-text);
  }

  /* sk-dropzone.active */
  .drop-zone--active {
    border-color: var(--sk-accent-active);
    background: rgba(var(--marble-active-rgb), 0.08);
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
    gap: var(--sk-space-xs);
  }

  .wip-meta span {
    color: var(--sk-muted);
    font-size: var(--sk-font-size-sm);
  }

  .wip-meta strong {
    color: var(--sk-text);
    font-family: var(--sk-font-mono);
    font-size: var(--sk-font-size-sm);
  }

  .loading-spinner {
    width: 30px;
    height: 30px;
    border: 3px solid rgba(var(--marble-vein-rgb), 0.1);
    border-top-color: var(--sk-accent-active);
    border-radius: var(--sk-radius-pill);
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
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
  }
</style>
