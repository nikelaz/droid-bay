import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  workers: 1,
  timeout: 40_000,
  use: {
    baseURL: "http://127.0.0.1:18081",
    viewport: { width: 1440, height: 1000 },
    trace: "retain-on-failure",
  },
  webServer: {
    command: "node scripts/test-server.mjs",
    url: "http://127.0.0.1:18081",
    reuseExistingServer: false,
    timeout: 15_000,
  },
});
