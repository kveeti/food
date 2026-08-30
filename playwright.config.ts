import { defineConfig, devices } from "@playwright/test";

const chromiumExecutablePath = Deno.env.get(
  "PLAYWRIGHT_CHROMIUM_EXECUTABLE",
);

export default defineConfig({
  testDir: "./tests",
  timeout: 20_000,
  use: {
    ...devices["iPhone 13"],
    baseURL: "http://127.0.0.1:8200",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: {
        browserName: "chromium",
        launchOptions: chromiumExecutablePath
          ? { executablePath: chromiumExecutablePath }
          : undefined,
      },
    },
  ],
  webServer: {
    command: "scripts/e2e-server.sh",
    url: "http://127.0.0.1:8200/",
    reuseExistingServer: false,
    timeout: 60_000,
    gracefulShutdown: { signal: "SIGTERM", timeout: 10_000 },
  },
});
