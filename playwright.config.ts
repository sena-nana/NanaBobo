import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  workers: 1,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:1431",
    browserName: "chromium",
    viewport: { width: 1280, height: 800 },
  },
  webServer: {
    command: "cross-env VITE_TEMPLATE_MOCK_FAILURE=1 yarn dev --host 127.0.0.1 --port 1431",
    url: "http://127.0.0.1:1431",
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
