import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: "**/*.spec.ts",
  timeout: 60_000,
  retries: 1,
  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  webServer: {
    command: "pnpm dev",
    port: 1420,
    reuseExistingServer: true,
    cwd: "../../",
    timeout: 30_000,
  },
  outputDir: "../test-artifacts/playwright",
});
