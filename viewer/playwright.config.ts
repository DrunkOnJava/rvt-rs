import { defineConfig, devices } from '@playwright/test';

// Overridable so two checkouts (e.g. a git worktree alongside the main
// clone) can run the suite at once without silently reusing each
// other's preview server on the default port.
const previewPort = Number(process.env.RVT_VIEWER_PORT ?? 4173);
const previewUrl = `http://127.0.0.1:${previewPort}`;

export default defineConfig({
  testDir: './tests',
  timeout: 120_000,
  expect: {
    timeout: 60_000,
  },
  fullyParallel: false,
  reporter: process.env.CI
    ? [
        ['list'],
        ['html', { open: 'never', outputFolder: 'playwright-report' }],
      ]
    : 'list',
  use: {
    baseURL: previewUrl,
    trace: 'retain-on-failure',
  },
  webServer: {
    command: `npm run preview -- --host 127.0.0.1 --port ${previewPort}`,
    url: previewUrl,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
