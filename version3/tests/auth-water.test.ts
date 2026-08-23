import { expect, test } from "@playwright/test";

import { chooseTimezone, ensureTimezone, login } from "./helpers";

const waterTotal = (page: import("@playwright/test").Page) =>
  page.getByLabel("Water total");
const waterProgress = (page: import("@playwright/test").Page) =>
  page.getByLabel("Water progress");

test("dev login moves to the configured host before setting cookies", async ({ page }) => {
  await page.goto("http://localhost:8200/");
  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();
  await expect(page).toHaveURL(/^http:\/\/127\.0\.0\.1:8200\/dev\/oidc\/authorize/);

  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await expect(page.getByRole("searchbox", { name: "Timezone" })).toHaveValue(
    "Europe/Helsinki",
  );
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

  const settingsRequestPromise = page.waitForRequest(
    (request) => new URL(request.url()).pathname === "/settings",
  );
  await page.getByRole("link", { name: "settings" }).click();
  const settingsRequest = await settingsRequestPromise;
  expect(settingsRequest.headers()["hx-request"]).toBe("true");
  let delaySearch = true;
  await page.route("**/settings/timezones?*", async (route) => {
    if (delaySearch) {
      delaySearch = false;
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
    await route.continue();
  });
  const timezone = page.getByRole("searchbox", { name: "Timezone" });
  await timezone.fill("America/New");
  await expect(page.locator(".search-spinner")).toBeVisible();
  await page.getByRole("button", { name: "America/New_York", exact: true }).click();

  await expect(page).toHaveURL("/settings");
  await expect(timezone).toHaveValue("America/New_York");

  await page.goto("/?date=2035-01-15");
  await expect(page.getByRole("heading", { name: "January 15, 2035" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Previous day" })).toHaveAttribute(
    "href",
    "/?date=2035-01-14",
  );
  const next = page.getByRole("link", { name: "Next day" });
  await expect(next).toHaveAttribute("href", "/?date=2035-01-16");
  const nextRequestPromise = page.waitForRequest(
    (request) => new URL(request.url()).searchParams.get("date") === "2035-01-16",
  );
  await next.click();
  const nextRequest = await nextRequestPromise;
  expect(nextRequest.headers()["hx-request"]).toBe("true");
  await expect(page).toHaveURL("/?date=2035-01-16");

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
  await expect(waterProgress(page)).toContainText("0 ml / no goal");

  await page.getByRole("button", { name: "250 ml" }).click();
  await expect(waterTotal(page)).toHaveText("250 ml");
  await expect(waterProgress(page)).toContainText("250 ml / no goal");
  await page.getByText("1 entry", { exact: true }).click();
  const history = page.locator("#water-history");
  await expect(page.getByRole("button", { name: "Delete 250 ml entry" })).toBeVisible();
  await page.getByRole("button", { name: "Delete 250 ml entry" }).click();
  await expect(history).toHaveAttribute("open", "");
  await expect(waterTotal(page)).toHaveText("0 ml");
  await expect(waterProgress(page)).toContainText("0 ml / no goal");

  await page.getByRole("button", { name: "250 ml" }).click();
  await expect(waterTotal(page)).toHaveText("250 ml");
  await page.getByRole("button", { name: "log out" }).click();
  await page.getByRole("link", { name: /bob@dev\.local/ }).click();
  await ensureTimezone(page);
  await expect(waterTotal(page)).toHaveText("0 ml");
  await expect(page.getByRole("button", { name: "Delete 250 ml entry" })).toHaveCount(0);
});

test("goals apply from their starting date", async ({ page }, testInfo) => {
  await login(page, testInfo);

  await page.getByRole("link", { name: "settings" }).click();
  await page.getByLabel("Baseline burn").fill("1800");
  await expect(page.getByLabel("By")).toBeHidden();
  await page.locator('select[name="adjustment_kind"]').selectOption("deficit");
  await expect(page.getByLabel("By")).toBeVisible();
  await page.getByLabel("By").fill("300");
  await page.getByLabel("Water").fill("2000");
  await page.getByLabel("Protein").fill("120");
  await page.getByLabel("Nutrient").fill("Sugars");
  await page.locator('input[name="add_nutrient_target"]').fill("50");
  const saveGoalsRequestPromise = page.waitForRequest(
    (request) => new URL(request.url()).pathname === "/settings/goals",
  );
  await page.getByRole("button", { name: "Save goals" }).click();
  const saveGoalsRequest = await saveGoalsRequestPromise;
  expect(saveGoalsRequest.headers()["hx-request"]).toBe("true");
  await page.getByRole("link", { name: "today" }).click();

  await expect(page.getByLabel("Food energy progress")).toContainText("1,500 kcal left");
  await expect(page.getByLabel("Food energy progress")).toContainText("0 / 1,500 kcal");
  await expect(waterProgress(page)).toContainText("2,000 ml left");
  await expect(waterProgress(page)).toContainText("0 / 2,000 ml");
  const nutrition = page.getByLabel("Daily nutrition totals");
  await expect(nutrition.getByText("Protein").locator(".."))
    .toContainText("0 / 120 g");
  await expect(nutrition.getByText("Sugars").locator(".."))
    .toContainText("0 / 50 g");

  await page.getByRole("link", { name: "settings" }).click();
  await page.getByLabel("Sugars").fill("0");
  await page.getByRole("button", { name: "Save goals" }).click();
  await page.getByRole("link", { name: "today" }).click();
  await expect(page.getByLabel("Daily nutrition totals")).not.toContainText("Sugars");

  await page.getByRole("link", { name: "Previous day" }).click();
  await expect(page.getByLabel("Food energy progress")).toContainText("0 kcal / no goal");
  await expect(waterProgress(page)).toContainText("0 ml / no goal");
  await expect(page.getByLabel("Daily nutrition totals")).not.toContainText("Sugars");
});

test("water progress shows amounts over the goal", async ({ page }, testInfo) => {
  await login(page, testInfo);

  await page.getByRole("link", { name: "settings" }).click();
  await page.locator('#goals input[name="water_goal_ml"]').fill("2000");
  await page.getByRole("button", { name: "Save goals" }).click();
  await page.getByRole("link", { name: "today" }).click();
  await page.getByText("Other amount", { exact: true }).click();
  await page.getByRole("spinbutton", { name: "Water in millilitres" }).fill("2250");
  await page.getByRole("button", { name: "Add", exact: true }).click();

  const progress = waterProgress(page);
  await expect(progress).toContainText("250 ml over");
  await expect(progress).toHaveAttribute("aria-valuemax", "2250");
  await expect(progress.locator('[data-goal-over="true"]')).toBeVisible();
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

  const history = page.locator("#water-history");
  await history.locator("summary").press("Enter");
  await page.getByRole("button", { name: "Delete 750 ml entry" }).click();
  await expect(waterTotal(page)).toHaveText("0 ml");
  await expect(history).toHaveAttribute("open", "");

  await context.close();
});
