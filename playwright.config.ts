import {defineConfig} from '@playwright/test';
export default defineConfig({
  testDir:'./tests/ui',
  use:{browserName:'chromium',channel:'msedge',baseURL:'http://127.0.0.1:1420',viewport:{width:420,height:260}},
  webServer:{command:'npm run dev',url:'http://127.0.0.1:1420',reuseExistingServer:!process.env.CI},
});
