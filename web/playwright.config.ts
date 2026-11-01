import { defineConfig, devices } from '@playwright/test';

const PORT = Number(process.env['UNTWIST_PORT'] ?? 4173);
const WEBKIT_WS = process.env['UNTWIST_WEBKIT_WS'];
const WEBKIT_NATIVE = process.env['UNTWIST_WEBKIT'] === '1';

export default defineConfig({
  testDir: 'e2e',
  timeout: 120_000,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: `http://localhost:${PORT}/`,
    viewport: { width: 1440, height: 960 },
    trace: 'off',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 960 }, headless: true, permissions: ['clipboard-read', 'clipboard-write'] },
    },
    {
      name: 'firefox',
      use: {
        ...devices['Desktop Firefox'],
        viewport: { width: 1440, height: 960 },
        headless: true,
        launchOptions: { firefoxUserPrefs: { 'dom.events.asyncClipboard.readText': true, 'dom.events.testing.asyncClipboard': true } },
      },
    },
    ...(WEBKIT_WS === undefined && !WEBKIT_NATIVE
      ? []
      : [
          {
            name: 'webkit',
            use: {
              ...devices['Desktop Safari'],
              viewport: { width: 1440, height: 960 },
              deviceScaleFactor: 1,
              permissions: ['clipboard-read'],
              ...(WEBKIT_WS === undefined ? { headless: true } : { connectOptions: { wsEndpoint: WEBKIT_WS } }),
            },
          },
        ]),
  ],
  webServer: {
    command: `npx vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}/`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
