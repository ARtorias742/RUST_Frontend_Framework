import { defineConfig, devices } from "@playwright/test";

// Ferrum's browser matrix gate: last 2 versions of Chromium, Firefox and
// WebKit. Run `npm install && npx playwright install` once, then
// `npm test` with a dev server on :8080 (e.g. `ferrum dev`).
export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: process.env.FERRUM_BASE_URL ?? "http://localhost:8080",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
  // Visual regression is only meaningful on one engine (font rasterisation
  // differs); keep it in its own project.
  snapshotPathTemplate: "{testDir}/__screenshots__/{testFilePath}/{arg}{ext}",
});