import { defineConfig } from '@playwright/test';
// Windows uses the installed Microsoft Edge. Elsewhere, point PLAYWRIGHT_CHROMIUM at a
// Chromium binary to run the same suite without Edge.
const chromium = process.env.PLAYWRIGHT_CHROMIUM;
export default defineConfig({
  testDir: './tests/ui',
  use: {
    browserName: 'chromium',
    ...(chromium ? { launchOptions: { executablePath: chromium } } : { channel: 'msedge' }),
    baseURL: 'http://127.0.0.1:1420',
    viewport: { width: 420, height: 260 },
  },
  webServer: {
    command: 'npm run dev',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: !process.env.CI,
  },
});
