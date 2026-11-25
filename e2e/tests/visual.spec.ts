import { test, expect } from "@playwright/test";

// Visual regression: pixel snapshots of the component gallery in light and
// dark themes. Baselines live in e2e/tests/__screenshots__/visual.spec.ts/.
// Update with `npm run test:visual -- --update-snapshots`.
//
// Prerequisites: the gallery example served on :8080.

test.describe("component gallery", () => {
  test("light theme matches the baseline", async ({ page }) => {
    await page.goto("/gallery");
    await expect(page.getByText("Ferrum Gallery")).toBeVisible();
    await expect(page).toHaveScreenshot("gallery-light.png", {
      maxDiffPixelRatio: 0.01,
    });
  });

  test("dark theme matches the baseline", async ({ page }) => {
    await page.goto("/gallery?theme=dark");
    await expect(page.getByText("Ferrum Gallery")).toBeVisible();
    await expect(page).toHaveScreenshot("gallery-dark.png", {
      maxDiffPixelRatio: 0.01,
    });
  });

  test("focus ring is visible for keyboard users", async ({ page }) => {
    await page.goto("/gallery");
    await page.keyboard.press("Tab");
    const focused = page.locator(":focus");
    await expect(focused).toBeVisible();
    await expect(focused).toHaveCSS("outline-style", /solid|auto/);
  });
});