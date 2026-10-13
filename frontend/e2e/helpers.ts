import type { Page } from "@playwright/test";

import { expect } from "./fixtures.ts";

export async function login(page: Page) {
  await page.goto("/");
  await expect(page).toHaveURL(/\/sign-in$/);
  await page.getByRole("link", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();
  await page.getByRole("link", { name: /alice/ }).click();
  await expect(page).toHaveURL(/\/settings$/);
}

export async function markDocument(page: Page) {
  await page.evaluate(() => {
    document.body.dataset.e2eDocument = "same";
  });
}

export async function expectSameDocument(page: Page) {
  await expect
    .poll(() => page.evaluate(() => document.body.dataset.e2eDocument))
    .toBe("same");
}

export async function finishSetup(page: Page) {
  await expect(page.getByLabel("Locale")).toHaveValue("en-US");
  await expect(page.getByLabel("Timezone")).toHaveValue("Europe/Helsinki");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page).toHaveURL((url) => url.pathname === "/");
}

export async function openWaterDrawer(page: Page) {
  await page.getByRole("button", { name: "Open water log" }).click();
  await expect(page.getByRole("dialog", { name: "Water" })).toBeVisible();
}

export async function closeWaterDrawer(page: Page) {
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Water" })).not.toBeVisible();
}
