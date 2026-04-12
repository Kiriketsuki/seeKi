import { defineConfig, devices } from '@playwright/test';
import path from 'path';

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
    {
      name: 'setup-wizard',
      testMatch: 'setup-wizard.spec.ts',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'normal',
      testMatch: ['data-grid.spec.ts', 'toolbar.spec.ts', 'navigation.spec.ts', 'error-states.spec.ts'],
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  globalSetup: path.resolve(__dirname, 'e2e/global-setup.ts'),
  globalTeardown: path.resolve(__dirname, 'e2e/global-teardown.ts'),

  timeout: 60_000,
  expect: {
    timeout: 10_000,
  },
});
