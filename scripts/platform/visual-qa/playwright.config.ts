import { defineConfig, devices } from '@playwright/test';

const port = Number(process.env.VQA_PORT ?? '4173');
const staticDir = process.env.VQA_STATIC_DIR;
const baseURL = process.env.VQA_BASE_URL ?? `http://127.0.0.1:${port}`;
const shellQuote = (value: string) => `'${value.replace(/'/g, `'\\''`)}'`;

const webServerCommand = process.env.VQA_WEB_SERVER_COMMAND
  ?? (staticDir
    ? `python3 -m http.server ${port} --bind 127.0.0.1 --directory ${shellQuote(staticDir)}`
    : undefined);

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  workers: 1,
  reporter: 'line',
  outputDir: 'tmp/playwright-results',
  use: {
    baseURL,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'off',
    viewport: {
      width: Number(process.env.VQA_VIEWPORT_WIDTH ?? '1440'),
      height: Number(process.env.VQA_VIEWPORT_HEIGHT ?? '960'),
    },
  },
  webServer: webServerCommand
    ? {
        command: webServerCommand,
        port,
        reuseExistingServer: false,
        timeout: 30_000,
      }
    : undefined,
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
