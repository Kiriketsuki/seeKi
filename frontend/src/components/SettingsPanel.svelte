<script lang="ts">
  import {
    X, RefreshCw, Download, RotateCcw, Upload,
    CircleCheck, AlertCircle,
  } from 'lucide-svelte';
  import type { UpdateStatus, VersionInfo, CheckResult, WipUploadResult } from '../lib/types';
  import {
    fetchVersion, fetchUpdateStatus, checkForUpdate,
    applyUpdate, uploadWipBinary, rollbackUpdate, updateSettings,
  } from '../lib/api';

  let {
    open = $bindable(false),
    initialStatus = null,
    onStatusChange,
  }: {
    open: boolean;
    initialStatus: UpdateStatus | null;
    onStatusChange?: (status: UpdateStatus) => void;
  } = $props();

  // ── State ──────────────────────────────────────────────────────────
  let status = $state<UpdateStatus | null>(null);
  let versionInfo = $state<VersionInfo | null>(null);
  let checkResult = $state<CheckResult | null>(null);
  let wipResult = $state<WipUploadResult | null>(null);

  let checking = $state(false);
  let applying = $state(false);
  let rollingBack = $state(false);
  let uploading = $state(false);
  let restarting = $state(false);
  let togglingChannel = $state(false);

  let errorMsg = $state<string | null>(null);
  let successMsg = $state<string | null>(null);

  let confirmAction = $state<'install' | 'rollback' | 'wip' | null>(null);

  let dragOver = $state(false);
  let fileInputEl = $state<HTMLInputElement | null>(null);
  let pollInterval = $state<ReturnType<typeof setInterval> | null>(null);

  // Sync initial status when panel opens; clean up on close
  $effect(() => {
    if (open) {
      status = initialStatus;
      errorMsg = null;
      successMsg = null;
      checkResult = null;
      wipResult = null;
      confirmAction = null;
      restarting = false;
      // Fetch fresh data
      fetchVersion()
        .then((v) => { versionInfo = v; })
        .catch((e) => {
          console.warn('Failed to fetch version info:', e);
        });
      fetchUpdateStatus()
        .then((s) => {
          status = s;
          onStatusChange?.(s);
        })
        .catch((e) => {
          console.warn('Failed to fetch update status:', e);
          errorMsg = 'Failed to load update status. Try again.';
        });
    }
    return () => {
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }
    };
  });

  // ── Derived ──────────────────────────────────────────────────────────
  let updateAvailable = $derived(status?.update_available ?? false);
  let previousExists = $derived(status?.previous_exists ?? false);
  let preReleaseChannel = $derived(status?.pre_release_channel ?? false);

  let lastCheckedLabel = $derived.by(() => {
    if (!status?.last_checked) return '—';
    try {
      const date = new Date(status.last_checked);
      return date.toLocaleString();
    } catch {
      return status.last_checked;
    }
  });

  // ── Handlers ──────────────────────────────────────────────────────────

  function close() {
    open = false;
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (confirmAction) {
        confirmAction = null;
      } else {
        close();
      }
    }
  }

  function handlePanelKeydown(e: KeyboardEvent) {
    if (e.key === 'Tab') {
      const panel = e.currentTarget as HTMLElement;
      const focusable = panel.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [tabindex]:not([tabindex="-1"]), input:not([type="hidden"]):not([tabindex="-1"])'
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  async function handleCheck() {
    checking = true;
    errorMsg = null;
    successMsg = null;
    checkResult = null;
    try {
      const result = await checkForUpdate();
      checkResult = result;
      // Refresh status
      const s = await fetchUpdateStatus();
      status = s;
      onStatusChange?.(s);
      if (result.update_available) {
        successMsg = `Update available: ${result.latest}`;
      } else {
        successMsg = 'You are running the latest version.';
      }
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : 'Update check failed';
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
      pollForRestart();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : 'Install failed';
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
      pollForRestart();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : 'Rollback failed';
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
      pollForRestart();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : 'WIP apply failed';
      applying = false;
    }
  }

  function pollForRestart() {
    if (pollInterval) clearInterval(pollInterval);
    let attempts = 0;
    const maxAttempts = 30; // 60 seconds
    pollInterval = setInterval(async () => {
      attempts++;
      try {
        await fetchVersion();
        if (pollInterval) clearInterval(pollInterval);
        pollInterval = null;
        window.location.reload();
      } catch {
        if (attempts >= maxAttempts) {
          if (pollInterval) clearInterval(pollInterval);
          pollInterval = null;
          restarting = false;
          applying = false;
          rollingBack = false;
          errorMsg = 'Server did not come back online. Please check manually.';
        }
      }
    }, 2000);
  }

  async function handleToggleChannel() {
    if (!status) return;
    togglingChannel = true;
    errorMsg = null;
    try {
      const next = !status.pre_release_channel;
      await updateSettings(next);
      status = { ...status, pre_release_channel: next };
      onStatusChange?.(status);
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : 'Failed to update settings';
    } finally {
      togglingChannel = false;
    }
  }

  function handleFileDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const file = e.dataTransfer?.files?.[0];
    if (file) doUpload(file);
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    dragOver = true;
  }

  function handleDragLeave() {
    dragOver = false;
  }

  function handleFileSelect(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (file) doUpload(file);
    input.value = '';
  }

  async function doUpload(file: File) {
    uploading = true;
    errorMsg = null;
    successMsg = null;
    wipResult = null;
    try {
      wipResult = await uploadWipBinary(file);
      successMsg = 'Binary uploaded successfully.';
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : 'Upload failed';
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

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="overlay" onclick={handleOverlayClick}>
    <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
    <aside class="panel" role="dialog" aria-label="Settings" aria-modal="true" onkeydown={handlePanelKeydown}>
      <!-- Header -->
      <div class="panel-header">
        <h2 class="panel-title">Settings</h2>
        <button class="close-btn" onclick={close} aria-label="Close settings">
          <X size={16} />
        </button>
      </div>

      <div class="panel-body">
        {#if restarting}
          <div class="restart-overlay">
            <div class="loading-spinner"></div>
            <p class="restart-text">Restarting...</p>
            <p class="restart-sub">Waiting for server to come back online</p>
          </div>
        {:else}
          <!-- ── Error / Success banners ── -->
          {#if errorMsg}
            <div class="banner banner-error">
              <AlertCircle size={14} />
              <span>{errorMsg}</span>
              <button class="banner-dismiss" onclick={() => errorMsg = null}>
                <X size={12} />
              </button>
            </div>
          {/if}
          {#if successMsg}
            <div class="banner banner-success">
              <CircleCheck size={14} />
              <span>{successMsg}</span>
              <button class="banner-dismiss" onclick={() => successMsg = null}>
                <X size={12} />
              </button>
            </div>
          {/if}

          <!-- ── Confirmation Dialog ── -->
          {#if confirmAction}
            <div class="confirm-card">
              <p class="confirm-text">
                {#if confirmAction === 'install'}
                  Install update {status?.latest ?? ''}? SeeKi will restart.
                {:else if confirmAction === 'rollback'}
                  Roll back to the previous version? SeeKi will restart.
                {:else if confirmAction === 'wip'}
                  Apply WIP binary ({wipResult ? formatBytes(wipResult.size) : ''})? SeeKi will restart.
                {/if}
              </p>
              <div class="confirm-actions">
                <button class="btn btn-ghost" onclick={() => confirmAction = null}>Cancel</button>
                <button
                  class="btn btn-accent"
                  onclick={() => {
                    if (confirmAction === 'install') handleInstall();
                    else if (confirmAction === 'rollback') handleRollback();
                    else if (confirmAction === 'wip') handleApplyWip();
                  }}
                >
                  Confirm
                </button>
              </div>
            </div>
          {/if}

          <!-- ── Updates section ── -->
          <section class="section">
            <h3 class="section-title">Updates</h3>

            <!-- Version info -->
            <div class="info-grid">
              <span class="info-label">Current version</span>
              <span class="info-value mono">
                {versionInfo?.version ?? status?.current ?? '—'}
                {#if versionInfo?.commit}
                  <span class="info-commit">({versionInfo.commit.slice(0, 7)})</span>
                {/if}
              </span>

              {#if versionInfo?.built_at}
                <span class="info-label">Built</span>
                <span class="info-value">{versionInfo.built_at}</span>
              {/if}

              <span class="info-label">Latest version</span>
              <span class="info-value mono">{status?.latest ?? '—'}</span>

              <span class="info-label">Last checked</span>
              <span class="info-value">{lastCheckedLabel}</span>
            </div>

            <!-- Status badge -->
            <div class="status-row">
              {#if updateAvailable}
                <span class="status-badge badge-update">
                  <AlertCircle size={12} />
                  Update available
                </span>
              {:else}
                <span class="status-badge badge-ok">
                  <CircleCheck size={12} />
                  Up to date
                </span>
              {/if}
            </div>

            <!-- Action buttons -->
            <div class="actions">
              <button
                class="btn btn-secondary"
                onclick={handleCheck}
                disabled={checking}
              >
                <RefreshCw size={14} class={checking ? 'spin' : ''} />
                {checking ? 'Checking...' : 'Check for updates'}
              </button>

              {#if updateAvailable}
                <button
                  class="btn btn-accent"
                  onclick={() => confirmAction = 'install'}
                  disabled={applying}
                >
                  <Download size={14} />
                  {applying ? 'Installing...' : 'Install update'}
                </button>
              {/if}

              <button
                class="btn btn-ghost"
                onclick={() => confirmAction = 'rollback'}
                disabled={!previousExists || rollingBack}
                title={previousExists ? 'Roll back to previous version' : 'No previous version available'}
              >
                <RotateCcw size={14} />
                {rollingBack ? 'Rolling back...' : 'Rollback'}
              </button>
            </div>

            <!-- Pre-release channel -->
            <div class="toggle-row">
              <div class="toggle-info">
                <span class="toggle-label">Pre-release channel</span>
                <span class="toggle-desc">Receive pre-release updates</span>
              </div>
              <button
                class="channel-toggle"
                class:active={preReleaseChannel}
                onclick={handleToggleChannel}
                disabled={togglingChannel}
                role="switch"
                aria-checked={preReleaseChannel}
                aria-label="Pre-release channel"
              >
                <span class="toggle-thumb"></span>
              </button>
            </div>
          </section>

          <!-- ── WIP Upload section ── -->
          <section class="section">
            <h3 class="section-title">WIP Upload</h3>
            <p class="section-desc">Upload a development binary for testing.</p>

            <div
              class="drop-zone"
              class:drag-over={dragOver}
              class:uploading
              ondrop={handleFileDrop}
              ondragover={handleDragOver}
              ondragleave={handleDragLeave}
              role="button"
              tabindex="0"
              aria-label="Upload WIP binary"
              onclick={() => fileInputEl?.click()}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') fileInputEl?.click(); }}
            >
              {#if uploading}
                <div class="loading-spinner small"></div>
                <span class="drop-text">Uploading...</span>
              {:else}
                <Upload size={20} />
                <span class="drop-text">Drop binary here or click to browse</span>
              {/if}
            </div>
            <input
              bind:this={fileInputEl}
              type="file"
              class="file-input-hidden"
              onchange={handleFileSelect}
              aria-hidden="true"
              tabindex="-1"
            />

            {#if wipResult}
              <div class="wip-info">
                <div class="info-grid compact">
                  <span class="info-label">SHA-256</span>
                  <span class="info-value mono small">{wipResult.sha256.slice(0, 16)}...</span>
                  <span class="info-label">Size</span>
                  <span class="info-value">{formatBytes(wipResult.size)}</span>
                </div>
                <button
                  class="btn btn-accent"
                  onclick={() => confirmAction = 'wip'}
                  disabled={applying}
                >
                  <Download size={14} />
                  Apply WIP
                </button>
              </div>
            {/if}
          </section>
        {/if}
      </div>
    </aside>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(47, 72, 88, 0.12);
    backdrop-filter: blur(2px);
    -webkit-backdrop-filter: blur(2px);
    display: flex;
    justify-content: flex-end;
  }

  .panel {
    width: 400px;
    max-width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--sk-glass-sidebar);
    backdrop-filter: var(--sk-glass-sidebar-blur);
    -webkit-backdrop-filter: var(--sk-glass-sidebar-blur);
    border-left: 1px solid var(--sk-border);
    box-shadow: -4px 0 24px rgba(47, 72, 88, 0.06);
    animation: slide-in 0.2s ease-out;
  }

  @keyframes slide-in {
    from { transform: translateX(100%); }
    to   { transform: translateX(0); }
  }

  /* ── Header ── */
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sk-space-lg) var(--sk-space-xl);
    border-bottom: 1px solid var(--sk-border-light);
    flex-shrink: 0;
  }

  .panel-title {
    font-size: var(--sk-font-size-lg);
    font-weight: 600;
    color: var(--sk-text);
    margin: 0;
  }

  .close-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: var(--sk-glass-button);
    color: var(--sk-muted);
    cursor: pointer;
    transition: color 0.15s ease, border-color 0.15s ease;
  }

  .close-btn:hover {
    color: var(--sk-text);
    border-color: var(--sk-border);
  }

  /* ── Body ── */
  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--sk-space-lg) var(--sk-space-xl);
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
  }

  /* ── Restart overlay ── */
  .restart-overlay {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-md);
    min-height: 200px;
  }

  .restart-text {
    font-size: var(--sk-font-size-lg);
    font-weight: 600;
    color: var(--sk-text);
    margin: 0;
  }

  .restart-sub {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    margin: 0;
  }

  /* ── Banners ── */
  .banner {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    border-radius: var(--sk-radius-md);
    font-size: var(--sk-font-size-sm);
    line-height: 1.4;
  }

  .banner span {
    flex: 1;
    min-width: 0;
  }

  .banner-error {
    background: rgba(220, 38, 38, 0.08);
    border: 1px solid rgba(220, 38, 38, 0.2);
    color: #b91c1c;
  }

  .banner-success {
    background: rgba(21, 128, 61, 0.08);
    border: 1px solid rgba(21, 128, 61, 0.2);
    color: #15803d;
  }

  .banner-dismiss {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    border-radius: var(--sk-radius-sm);
    background: none;
    color: inherit;
    cursor: pointer;
    flex-shrink: 0;
    opacity: 0.6;
  }

  .banner-dismiss:hover {
    opacity: 1;
  }

  /* ── Confirm card ── */
  .confirm-card {
    padding: var(--sk-space-md) var(--sk-space-lg);
    background: rgba(255, 149, 0, 0.06);
    border: 1px solid rgba(255, 149, 0, 0.2);
    border-radius: var(--sk-radius-md);
  }

  .confirm-text {
    font-size: var(--sk-font-size-body);
    color: var(--sk-text);
    margin: 0 0 var(--sk-space-md);
    line-height: 1.5;
  }

  .confirm-actions {
    display: flex;
    gap: var(--sk-space-sm);
    justify-content: flex-end;
  }

  /* ── Sections ── */
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-md);
  }

  .section-title {
    font-size: var(--sk-font-size-md);
    font-weight: 600;
    color: var(--sk-text);
    margin: 0;
  }

  .section-desc {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    margin: 0;
  }

  /* ── Info grid ── */
  .info-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--sk-space-xs) var(--sk-space-lg);
    align-items: baseline;
  }

  .info-grid.compact {
    gap: var(--sk-space-xs) var(--sk-space-md);
  }

  .info-label {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    white-space: nowrap;
  }

  .info-value {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-text);
    word-break: break-all;
  }

  .info-value.mono,
  .mono {
    font-family: var(--sk-font-mono);
    font-size: var(--sk-font-size-sm);
  }

  .info-value.small {
    font-size: var(--sk-font-size-xs);
  }

  .info-commit {
    color: var(--sk-muted);
    font-size: var(--sk-font-size-xs);
  }

  /* ── Status badge ── */
  .status-row {
    display: flex;
    align-items: center;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: 3px var(--sk-space-sm);
    border-radius: 100px;
    font-size: var(--sk-font-size-sm);
    font-weight: 500;
  }

  .badge-ok {
    background: rgba(21, 128, 61, 0.1);
    color: #15803d;
  }

  .badge-update {
    background: rgba(255, 149, 0, 0.12);
    color: #c27400;
  }

  /* ── Buttons ── */
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sk-space-sm);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--sk-space-xs);
    padding: var(--sk-space-xs) var(--sk-space-md);
    border-radius: var(--sk-radius-md);
    font-family: var(--sk-font-ui);
    font-size: var(--sk-font-size-sm);
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease, opacity 0.15s ease;
    white-space: nowrap;
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--sk-glass-button);
    backdrop-filter: var(--sk-glass-button-blur);
    -webkit-backdrop-filter: var(--sk-glass-button-blur);
    border-color: var(--sk-border-light);
    color: var(--sk-text);
  }

  .btn-secondary:hover:not(:disabled) {
    border-color: var(--sk-border);
    background: rgba(255, 255, 255, 0.8);
  }

  .btn-accent {
    background: var(--sk-accent);
    color: white;
    border-color: var(--sk-accent);
  }

  .btn-accent:hover:not(:disabled) {
    background: #e68600;
    border-color: #e68600;
  }

  .btn-ghost {
    background: none;
    border-color: var(--sk-border-light);
    color: var(--sk-muted);
  }

  .btn-ghost:hover:not(:disabled) {
    color: var(--sk-text);
    border-color: var(--sk-border);
    background: var(--sk-glass-button);
  }

  /* ── Toggle switch ── */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sk-space-md);
    padding: var(--sk-space-sm) 0;
    border-top: 1px solid var(--sk-border-lighter);
  }

  .toggle-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .toggle-label {
    font-size: var(--sk-font-size-sm);
    color: var(--sk-text);
    font-weight: 500;
  }

  .toggle-desc {
    font-size: var(--sk-font-size-xs);
    color: var(--sk-muted);
  }

  .channel-toggle {
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: 10px;
    border: 1px solid var(--sk-border);
    background: rgba(0, 0, 0, 0.06);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.2s ease, border-color 0.2s ease;
    padding: 0;
  }

  .channel-toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .channel-toggle.active {
    background: var(--sk-accent);
    border-color: var(--sk-accent);
  }

  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: white;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
    transition: transform 0.2s ease;
  }

  .channel-toggle.active .toggle-thumb {
    transform: translateX(16px);
  }

  /* ── Drop zone ── */
  .drop-zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-xl) var(--sk-space-lg);
    border: 2px dashed var(--sk-border);
    border-radius: var(--sk-radius-lg);
    color: var(--sk-muted);
    cursor: pointer;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .drop-zone:hover,
  .drop-zone.drag-over {
    border-color: var(--sk-accent);
    background: rgba(255, 149, 0, 0.04);
  }

  .drop-zone.uploading {
    pointer-events: none;
    opacity: 0.7;
  }

  .drop-text {
    font-size: var(--sk-font-size-sm);
    text-align: center;
  }

  .file-input-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  /* ── WIP info ── */
  .wip-info {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
    padding: var(--sk-space-sm) var(--sk-space-md);
    background: rgba(255, 255, 255, 0.4);
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-md);
  }

  /* ── Spinner ── */
  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--sk-border);
    border-top-color: var(--sk-accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .loading-spinner.small {
    width: 16px;
    height: 16px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Spin class for the check icon */
  :global(.spin) {
    animation: spin 0.8s linear infinite;
  }
</style>
