import { defineConfig } from "@playwright/test";

const baseURL = process.env.RIVET_TEST_URL || "http://127.0.0.1:7332";

export default defineConfig({
  testDir: "./tests/ui",
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  expect: { timeout: 15_000 },
  reporter: "list",
  use: {
    baseURL,
    browserName: "chromium",
    viewport: { width: 1440, height: 1100 },
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  webServer: process.env.RIVET_TEST_URL
    ? undefined
    : {
        command: "cargo run --locked -- ui --port 7332",
        url: baseURL,
        timeout: 120_000,
        reuseExistingServer: false,
        stdout: "pipe",
        stderr: "pipe",
      },
});
