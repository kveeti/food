import { expect, test } from "@playwright/test";

import { login } from "./helpers";

test("dev login moves to the configured host before setting cookies", async ({ page }) => {
  await page.goto("http://localhost:8200/");
  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();
  await expect(page).toHaveURL(/^http:\/\/127\.0\.0\.1:8200\/dev\/oidc\/authorize/);

  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await expect(page).toHaveURL("http://127.0.0.1:8200/");
});

test("OIDC login creates a session and logout ends it", async ({
  page,
}, testInfo) => {
  await login(page, testInfo);

  await page.getByRole("button", { name: "log out" }).click();
  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();

  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();
});

test("water logging uses htmx and stays private to the user", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  const total = page.getByRole("region", { name: "Water total" }).locator("p");
  await expect(total).toHaveText("0ml today");

  await page.getByRole("button", { name: "250 ml" }).click();
  await expect(total).toHaveText("250ml today");
  await expect(page.getByRole("button", { name: "Delete 250 ml entry" })).toBeVisible();

  await page.getByRole("button", { name: "log out" }).click();
  await page.getByRole("link", { name: /bob@dev\.local/ }).click();
  await expect(total).toHaveText("0ml today");
  await expect(page.getByRole("button", { name: "Delete 250 ml entry" })).toHaveCount(0);
});

test("login and water forms work without JavaScript", async ({ browser }, testInfo) => {
  const context = await browser.newContext({
    baseURL: "http://127.0.0.1:8200",
    javaScriptEnabled: false,
  });
  const page = await context.newPage();

  await login(page, testInfo);
  const total = page.getByRole("region", { name: "Water total" }).locator("p");
  await page.getByRole("button", { name: "750 ml" }).click();
  await expect(total).toHaveText("750ml today");

  await page.getByRole("button", { name: "Delete 750 ml entry" }).click();
  await expect(total).toHaveText("0ml today");

  await context.close();
});
