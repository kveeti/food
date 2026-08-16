import { expect, test } from "@playwright/test";

test("logs in and out through OIDC", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Pick a dev user" })).toBeVisible();
  await page.getByRole("link", { name: /alice/ }).click();

  await expect(page).toHaveURL("/");
  await page.waitForLoadState("load");
  await page.getByRole("button", { name: "Open menu" }).click();
  await expect(page.getByText("alice@dev.local", { exact: true })).toBeVisible();
  await page.getByRole("menuitem", { name: "Log out" }).click();
  await expect(page).toHaveURL(/\/dev\/oidc\/authorize\?/);
  await expect(page.getByRole("heading", { name: "Pick a dev user" })).toBeVisible();
});

test("opens the profile", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: /alice/ }).click();
  await expect(page).toHaveURL("/");
  await page.waitForLoadState("load");
  await page.getByRole("button", { name: "Open menu" }).click();
  await page.getByRole("menuitem", { name: /alice@dev\.local/ }).click();

  await expect(page).toHaveURL("/profile");
  await expect(page.getByRole("heading", { name: "Profile" })).toBeVisible();
  await expect(page.getByRole("definition")).toHaveText("alice@dev.local");
});

test("can create a dev user", async ({ page }) => {
  await page.goto("/auth/login");
  await page.getByPlaceholder("new-user-sub").fill("playwright-user");
  await page.getByRole("button", { name: "Log in as new user" }).click();
  await page.waitForLoadState("load");
  await page.getByRole("button", { name: "Open menu" }).click();

  await expect(page.getByText("playwright-user@dev.local", { exact: true })).toBeVisible();
});
