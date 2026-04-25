<script lang="ts">
  import type { WizardData } from '../lib/types';
  import SetupStep1Connection from './SetupStep1Connection.svelte';
  import SetupStep2Tables from './SetupStep2Tables.svelte';
  import SetupStep3Branding from './SetupStep3Branding.svelte';
  import SetupStep4Confirm from './SetupStep4Confirm.svelte';

  const STEP_NAMES = [
    'Connect your database',
    'Choose tables',
    'Brand your app',
    'Confirm & save',
  ];
  const TOTAL_STEPS = 4;

  let currentStep: number = $state(1);

  let wizardData: WizardData = $state({
    connection_mode: 'url',
    url: '',
    host: 'localhost',
    port: 5432,
    database: 'postgres',
    db_user: '',
    db_password: '',
    use_ssh: false,
    ssh: {
      host: '',
      port: 22,
      username: '',
      auth_method: 'key',
      key_path: '',
      key_passphrase: '',
      password: '',
    },
    tables: [],
    schemas: [],
    selected_tables: [],
    selected_schemas: [],
    title: 'SeeKi',
    subtitle: '',
  });

  function goNext() {
    if (currentStep < TOTAL_STEPS) currentStep++;
  }

  function goBack() {
    if (currentStep > 1) currentStep--;
  }

  function goToStep(step: number) {
    if (step >= 1 && step <= TOTAL_STEPS) currentStep = step;
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label="Setup wizard">
  <div class="card">

    <!-- Progress dots -->
    <div class="progress" role="group" aria-label="Setup progress">
      {#each { length: TOTAL_STEPS } as _, i}
        {@const step = i + 1}
        <button
          class="dot"
          class:current={step === currentStep}
          class:completed={step < currentStep}
          onclick={() => { if (step < currentStep) goToStep(step); }}
          aria-label="Step {step}: {STEP_NAMES[i]}{step < currentStep ? ' (completed)' : step === currentStep ? ' (current)' : ''}"
          aria-current={step === currentStep ? 'step' : undefined}
          tabindex={step < currentStep ? 0 : -1}
        ></button>
      {/each}
    </div>

    <!-- Step header -->
    <div class="step-header">
      <p class="step-label">Step {currentStep} of {TOTAL_STEPS}</p>
      <h2 class="step-title">{STEP_NAMES[currentStep - 1]}</h2>
    </div>

    <!-- Step content -->
    <div class="step-content">
      {#if currentStep === 1}
        <SetupStep1Connection bind:wizardData onNext={goNext} />
      {:else if currentStep === 2}
        <SetupStep2Tables bind:wizardData onNext={goNext} onBack={goBack} />
      {:else if currentStep === 3}
        <SetupStep3Branding bind:wizardData onNext={goNext} onBack={goBack} />
      {:else if currentStep === 4}
        <SetupStep4Confirm {wizardData} onBack={goBack} onGoToStep={goToStep} />
      {/if}
    </div>

  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(15, 25, 35, 0.92);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sk-space-lg);
  }

  .card {
    background: rgba(255, 255, 255, 0.92);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border-radius: var(--sk-radius-lg);
    padding: 40px;
    max-width: 520px;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xl);
    box-shadow: 0 24px 80px rgba(15,25,35,0.3), 0 4px 16px rgba(15,25,35,0.15);
    max-height: calc(100vh - 48px);
    overflow-y: auto;
  }

  /* Progress dots */
  .progress {
    display: flex;
    align-items: center;
    gap: var(--sk-space-sm);
    justify-content: center;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: none;
    background: rgba(47, 72, 88, 0.15);
    padding: 0;
    transition: background 0.2s, transform 0.2s, width 0.2s;
    cursor: default;
  }
  .dot.completed {
    background: var(--sk-muted);
    cursor: pointer;
  }
  .dot.completed:hover {
    background: var(--sk-secondary-strong);
    transform: scale(1.2);
  }
  .dot.current {
    background: var(--sk-accent);
    width: 24px;
    border-radius: var(--sk-radius-sm);
  }

  /* Step header */
  .step-header {
    display: flex;
    flex-direction: column;
    gap: var(--sk-space-xs);
  }
  .step-label {
    margin: 0;
    font-size: var(--sk-font-size-sm);
    color: var(--sk-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 500;
  }
  .step-title {
    margin: 0;
    font-size: var(--sk-font-size-xl);
    font-weight: 700;
    color: var(--sk-text);
    line-height: 1.2;
  }

  /* Step content area */
  .step-content {
    flex: 1;
  }
</style>
