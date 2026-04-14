<script lang="ts">
  import { Settings } from 'lucide-svelte';

  let {
    updateAvailable = false,
    onclick,
  }: {
    updateAvailable: boolean;
    onclick: () => void;
  } = $props();
</script>

<button class="gear-btn" {onclick} aria-label="Settings">
  <Settings size={16} />
  {#if updateAvailable}
    <span class="badge" aria-label="Update available"></span>
  {/if}
</button>

<style>
  .gear-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--sk-border-light);
    border-radius: var(--sk-radius-sm);
    background: var(--sk-glass-button);
    backdrop-filter: var(--sk-glass-button-blur);
    -webkit-backdrop-filter: var(--sk-glass-button-blur);
    color: var(--sk-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: color 0.15s ease, border-color 0.15s ease, background 0.15s ease;
  }

  .gear-btn:hover {
    color: var(--sk-text);
    border-color: var(--sk-border);
    background: rgba(255, 255, 255, 0.75);
  }

  .badge {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 8px;
    height: 8px;
    background: var(--sk-accent);
    border-radius: 50%;
    border: 1.5px solid var(--sk-bg);
    animation: badge-pulse 2s ease-in-out infinite;
  }

  @keyframes badge-pulse {
    0%, 100% {
      box-shadow: 0 0 0 0 rgba(255, 149, 0, 0.4);
    }
    50% {
      box-shadow: 0 0 0 4px rgba(255, 149, 0, 0);
    }
  }
</style>
