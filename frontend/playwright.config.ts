import { defineConfig, devices } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const BASE_URL = 'http://127.0.0.1:3141';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [['html', { open: 'never' }], ['list']],

  use: {
    baseURL: BASE_URL,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'off',
  },

  projects: [
    // --- Chromium (always runs) ---
    {
      name: 'setup-wizard',
      testMatch: 'setup-wizard.spec.ts',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'normal',
      testMatch: ['data-grid.spec.ts', 'action-dock.spec.ts', 'navigation.spec.ts', 'error-states.spec.ts'],
      use: { ...devices['Desktop Chrome'] },
    },
    // --- Firefox (opt-in: SEEKI_ALL_BROWSERS=1 or --project=normal-firefox) ---
    ...(process.env.SEEKI_ALL_BROWSERS === '1' ? [
      {
        name: 'setup-wizard-firefox',
        testMatch: 'setup-wizard.spec.ts',
        use: { ...devices['Desktop Firefox'] },
      },
      {
        name: 'normal-firefox',
        testMatch: ['data-grid.spec.ts', 'action-dock.spec.ts', 'navigation.spec.ts', 'error-states.spec.ts'],
        use: { ...devices['Desktop Firefox'] },
      },
    ] : []),
    // --- WebKit / Safari (opt-in: SEEKI_ALL_BROWSERS=1 or --project=normal-webkit) ---
    ...(process.env.SEEKI_ALL_BROWSERS === '1' ? [
      {
        name: 'setup-wizard-webkit',
        testMatch: 'setup-wizard.spec.ts',
        use: { ...devices['Desktop Safari'] },
      },
      {
        name: 'normal-webkit',
        testMatch: ['data-grid.spec.ts', 'action-dock.spec.ts', 'navigation.spec.ts', 'error-states.spec.ts'],
        use: { ...devices['Desktop Safari'] },
      },
    ] : []),
  ],

  globalSetup: path.resolve(__dirname, 'e2e/global-setup.ts'),
  globalTeardown: path.resolve(__dirname, 'e2e/global-teardown.ts'),

  timeout: 60_000,
  expect: {
    timeout: 10_000,
  },
});
