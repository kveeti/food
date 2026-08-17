import { expect, test } from "@playwright/test";

import { chooseTimezone, ensureTimezone, login } from "./helpers";

const waterTotal = (page: import("@playwright/test").Page) =>
  page.getByLabel("Water total");

test("dev login moves to the configured host before setting cookies", async ({ page }) => {
  await page.goto("http://localhost:8200/");
  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();
  await expect(page).toHaveURL(/^http:\/\/127\.0\.0\.1:8200\/dev\/oidc\/authorize/);

  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await chooseTimezone(page);
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

test("settings stores a timezone and days can move backward and forward", async ({
  page,
}, testInfo) => {
  await login(page, testInfo);

  await page.getByRole("link", { name: "settings" }).click();
  await page
    .getByRole("combobox", { name: "Timezone" })
    .selectOption("America/New_York");
  await page.getByRole("button", { name: "Save" }).click();
  await page.getByRole("link", { name: "settings" }).click();
  await expect(page.getByRole("combobox", { name: "Timezone" })).toHaveValue(
    "America/New_York",
  );

  await page.goto("/?date=2035-01-15");
  await expect(page.getByRole("heading", { name: "January 15, 2035" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Previous day" })).toHaveAttribute(
    "href",
    "/?date=2035-01-14",
  );
  await expect(page.getByRole("link", { name: "Next day" })).toHaveAttribute(
    "href",
    "/?date=2035-01-16",
  );

  await page.getByRole("button", { name: "250 ml" }).click();
  await expect(waterTotal(page)).toHaveText("250 ml");
  await page.getByRole("link", { name: "today" }).click();
  await expect(waterTotal(page)).toHaveText("0 ml");
});

test("water logging uses htmx and stays private to the user", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await ensureTimezone(page);
  await expect(waterTotal(page)).toHaveText("0 ml");

  await page.getByRole("button", { name: "250 ml" }).click();
  await expect(waterTotal(page)).toHaveText("250 ml");
  await page.getByText("1 entry", { exact: true }).click();
  const history = page.locator("#water-history");
  await expect(page.getByRole("button", { name: "Delete 250 ml entry" })).toBeVisible();
  await page.getByRole("button", { name: "Delete 250 ml entry" }).click();
  await expect(history).toHaveAttribute("open", "");
  await expect(waterTotal(page)).toHaveText("0 ml");

  await page.getByRole("button", { name: "250 ml" }).click();
  await expect(waterTotal(page)).toHaveText("250 ml");
  await page.getByRole("button", { name: "log out" }).click();
  await page.getByRole("link", { name: /bob@dev\.local/ }).click();
  await ensureTimezone(page);
  await expect(waterTotal(page)).toHaveText("0 ml");
  await expect(page.getByRole("button", { name: "Delete 250 ml entry" })).toHaveCount(0);
});

test("login and water forms work without JavaScript", async ({ browser }, testInfo) => {
  const context = await browser.newContext({
    baseURL: "http://127.0.0.1:8200",
    javaScriptEnabled: false,
  });
  const page = await context.newPage();

  await login(page, testInfo);
  await page.getByRole("button", { name: "750 ml" }).click();
  await expect(waterTotal(page)).toHaveText("750 ml");

  await page.getByText("1 entry", { exact: true }).click();
  await page.getByRole("button", { name: "Delete 750 ml entry" }).click();
  await expect(waterTotal(page)).toHaveText("0 ml");

  await context.close();
});
