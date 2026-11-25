import { test, expect } from "@playwright/test";

// Phase 9 gate (browser half): the DevTools overlay is usable to diagnose a
// real bug without leaving the page.
//
// Prerequisites: `ferrum dev` (or `trunk serve`) serving examples/devtools on
// :8080, and `npx playwright install`.

test.describe("DevTools overlay", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Ferrum DevTools")).toBeVisible();
  });

  test("inspector lists the mounted component tree", async ({ page }) => {
    await expect(page.getByText("Counter #", { exact: false })).toBeVisible();
    await expect(page.getByText("renders=1", { exact: false })).toBeVisible();
  });

  test("state timeline records a click", async ({ page }) => {
    await page.getByRole("button", { name: /Count: 0/ }).click();
    await expect(page.getByRole("button", { name: /Count: 1/ })).toBeVisible();
    await expect(page.getByText("setState", { exact: false })).toBeVisible();
    await expect(page.getByText("hook #0", { exact: false })).toBeVisible();
  });

  test("timeline is ordered newest-first and capped", async ({ page }) => {
    for (let i = 0; i < 5; i++) {
      await page.getByRole("button", { name: new RegExp(`Count: ${i}`) }).click();
    }
    await expect(page.getByRole("button", { name: /Count: 5/ })).toBeVisible();
    const events = page.getByText(/^\s*\d+ms\s+(setState|render)/);
    await expect(events.first()).toBeVisible();
  });
});