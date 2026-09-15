import type { Page } from "@playwright/test";

import { expect, test } from "./fixtures.ts";
import { finishSetup } from "./helpers.ts";

test.skip(process.env.E2E_PREVIEW !== "1", "Requires the production build");
test.describe.configure({ mode: "default" });

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("shows sign-in and opens the login provider", async ({ page }) => {
    await page.goto("/sign-in");
    await expect(
      page.getByRole("heading", { name: "Sign in to Food" }),
    ).toBeVisible();

    await page.getByRole("link", { name: "Sign in", exact: true }).click();
    await expect(
      page.getByRole("heading", { name: "Pick a dev user" }),
    ).toBeVisible();
  });
});

test("hydrates sign-in and completes login and logout", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (
      message.type() === "error" &&
      /hydrat|Minified React error/i.test(message.text())
    ) {
      errors.push(message.text());
    }
  });

  const waitForPreload = await trackIdlePreload(page);
  await page.goto("/sign-in");
  await waitForPreload();
  await page.getByRole("link", { name: "Sign in", exact: true }).click();
  await page.getByRole("link", { name: /alice/ }).click();
  await expect(page).toHaveURL(/\/settings$/);
  await finishSetup(page);

  await page.getByRole("link", { name: "You", exact: true }).click();
  await page.getByRole("button", { name: "Log out", exact: true }).click();
  await expect(page).toHaveURL(/\/sign-in$/);
  await expect(
    page.getByRole("heading", { name: "Sign in to Food" }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

for (const [name, connection, shouldPreload] of [
  ["Data Saver", { saveData: true, effectiveType: "4g" }, false],
  ["3g", { saveData: false, effectiveType: "3g" }, false],
  ["4g", { saveData: false, effectiveType: "4g" }, true],
  ["no connection API", null, true],
] as const) {
  test(`preloading with ${name}`, async ({ page }) => {
    await page.addInitScript((connection) => {
      Object.defineProperty(navigator, "connection", {
        value: connection ?? undefined,
      });
    }, connection);

    const waitForPreload = await trackIdlePreload(page);
    const loadedPages = new Set<string>();
    const buildOnlyRequests: string[] = [];
    page.on("request", (request) => {
      const path = new URL(request.url()).pathname;
      const match = path.match(
        /\/(home-page|settings-page|log-page)-[^/]+\.js$/,
      );
      if (match) loadedPages.add(match[1]);
      if (/\/(prerender|react-server)-/.test(path)) {
        buildOnlyRequests.push(path);
      }
    });

    await page.goto("/sign-in");
    await waitForPreload();
    if (shouldPreload) {
      await expect.poll(() => loadedPages.size).toBe(3);
    } else {
      expect(loadedPages.size).toBe(0);
    }
    expect(buildOnlyRequests).toEqual([]);
    await expect(
      page.getByRole("heading", { name: "Sign in to Food" }),
    ).toBeVisible();
  });
}

async function trackIdlePreload(page: Page) {
  let isIdleFinished = false;
  await page.exposeFunction("reportIdlePreload", () => {
    isIdleFinished = true;
  });
  await page.addInitScript(() => {
    const requestIdleCallback = window.requestIdleCallback.bind(window);
    window.requestIdleCallback = (callback, options) =>
      requestIdleCallback((deadline) => {
        callback(deadline);
        void (
          window as Window & { reportIdlePreload?: () => Promise<void> }
        ).reportIdlePreload?.();
      }, options);
  });

  return () => expect.poll(() => isIdleFinished).toBe(true);
}
