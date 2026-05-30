<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    description,
    children,
  }: {
    title: string;
    description: string;
    children?: Snippet;
  } = $props();
</script>

<section class="panel-frame">
  <header class="panel-header">
    <h1>{title}</h1>
    <p>{description}</p>
  </header>

  {#if children}
    <div class="panel-body">
      {@render children()}
    </div>
  {/if}
</section>

<style>
  /* sk-pf in the design reference */
  .panel-frame {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xl);
  }

  /* sk-pf-head */
  .panel-header {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-sm);
  }

  .panel-header h1 {
    font-size: var(--sk-font-size-xl);
    font-weight: 600;
    color: var(--sk-text);
    letter-spacing: -0.01em;
    margin: 0;
  }

  .panel-header p {
    max-width: 70ch;
    color: var(--sk-secondary-strong);
    line-height: 1.6;
    margin: 0;
  }

  /* sk-pf-body */
  .panel-body {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-lg);
  }
</style>
