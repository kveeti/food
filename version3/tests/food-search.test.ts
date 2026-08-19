import { expect, test } from "@playwright/test";

import { chooseTimezone, login } from "./helpers";

test("food search previews scaled nutrients", async ({ page }, testInfo) => {
  await login(page, testInfo);

  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  const result = page.getByRole("link", { name: /SOKERI/ });
  await expect(result).toBeVisible();
  await expect(result).toContainText("406 kcal");
  await result.click();

  await expect(page.getByRole("heading", { name: "SOKERI" })).toBeVisible();
  await page.getByRole("spinbutton", { name: "Amount in grams" }).fill("50");
  await expect(page.getByText("203 kcal", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Log food" })).toBeVisible();

  await page.getByRole("searchbox", { name: "Search foods" }).fill("nekta");
  await expect(
    page.getByRole("link", { name: /Persikka\/nektariini/ }),
  ).toBeVisible();

  await page.getByRole("searchbox", { name: "Search foods" }).fill("maito");
  await expect(page.locator("#food-results a").first()).toContainText(
    "Maito, rasvaton",
  );

  await page.getByRole("searchbox", { name: "Search foods" }).fill("maito rasvaton");
  await expect(page.locator("#food-results a").first()).toContainText(
    "Maito, rasvaton",
  );

  await page.getByRole("searchbox", { name: "Search foods" }).fill("rasvaton maito");
  await expect(page.locator("#food-results a").first()).toContainText(
    "Maito, rasvaton",
  );

  await page.getByRole("searchbox", { name: "Search foods" }).fill("omena");
  await expect(page.locator("#food-results a").first()).toContainText(
    "Omena, keskiarvo",
  );

  await page.getByRole("searchbox", { name: "Search foods" }).fill("apple");
  await expect(page.locator("#food-results a").first()).toContainText(
    "Omena, keskiarvo",
  );

  await page.getByRole("searchbox", { name: "Search foods" }).fill("salmon");
  await expect(page.locator("#food-results a").first()).toContainText("Lohi");
  await expect(page.getByRole("link", { name: /Taimen/ })).toHaveCount(0);

  await page.getByRole("searchbox", { name: "Search foods" }).fill("mjölk");
  await expect(page.locator("#food-results a").first()).toContainText(
    "Maito, rasvaton",
  );

  await page.getByRole("searchbox", { name: "Search foods" }).fill("socker");
  await expect(page.locator("#food-results a").first()).toContainText("SOKERI");

  await page.getByRole("searchbox", { name: "Search foods" }).fill("rainbow");
  await expect(page.locator("#food-results a").first()).toContainText("Honung");
});

test("food logging snapshots nutrients and continues the active meal", async ({ page }, testInfo) => {
  await login(page, testInfo);

  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  await page.getByRole("link", { name: /SOKERI/ }).click();
  await page.getByRole("spinbutton", { name: "Amount in grams" }).fill("50");
  await expect(page.getByText("203 kcal", { exact: true })).toBeVisible();
  await page.getByLabel("New meal name (optional)").fill("Lunch");
  await page.getByRole("button", { name: "Log food" }).click();

  await expect(page.getByLabel("Food energy total")).toHaveText("203 kcal");
  await expect(page.getByRole("heading", { name: /^Lunch ·/ })).toBeVisible();
  await page.getByText("SOKERI", { exact: true }).click();
  await page.getByRole("spinbutton", { name: "Amount in grams" }).fill("100");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByLabel("Food energy total")).toHaveText("406 kcal");

  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  await page.getByRole("link", { name: /SOKERI/ }).click();
  await expect(page.locator('select[name="meal"]')).not.toHaveValue("new");
  await page.getByRole("spinbutton", { name: "Amount in grams" }).fill("25");
  await page.getByRole("button", { name: "Log food" }).click();

  await expect(page.getByRole("heading", { name: /^Lunch ·/ })).toHaveCount(1);
  await expect(page.getByText("SOKERI", { exact: true })).toHaveCount(2);
  await expect(page.getByLabel("Food energy total")).toHaveText("507 kcal");

  await page.getByText("SOKERI", { exact: true }).last().click();
  await page.getByRole("button", { name: "Delete SOKERI" }).last().click();
  await expect(page.getByLabel("Food energy total")).toHaveText("406 kcal");
  await page.getByText("SOKERI", { exact: true }).click();
  await page.getByRole("button", { name: "Delete SOKERI" }).click();
  await expect(page.getByLabel("Food energy total")).toHaveText("0 kcal");
  await expect(page.getByText("No food logged")).toBeVisible();
});

test("a new meal becomes the only meal that can be continued", async ({ page }, testInfo) => {
  await login(page, testInfo);

  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  await page.getByRole("link", { name: /SOKERI/ }).click();
  await page.getByLabel("New meal name (optional)").fill("Lunch");
  await page.getByRole("button", { name: "Log food" }).click();

  await page.getByRole("searchbox", { name: "Search foods" }).fill("peach");
  await page.getByRole("link", { name: /Persikka\/nektariini/ }).click();
  await page.locator('select[name="meal"]').selectOption("new");
  await page.getByLabel("New meal name (optional)").fill("Snack");
  await page.getByRole("button", { name: "Log food" }).click();
  await expect(page.getByText("* Incomplete", { exact: true })).toBeVisible();

  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  await page.getByRole("link", { name: /SOKERI/ }).click();
  const meal = page.locator('select[name="meal"]');
  await expect(meal.locator("option")).toContainText([
    "Start a new meal",
    /Continue Snack ·/,
  ]);
  await expect(meal).not.toContainText("Continue Lunch");
});

test("food entries stay private to their owner", async ({ page }, testInfo) => {
  await login(page, testInfo);
  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  await page.getByRole("link", { name: /SOKERI/ }).click();
  await page.getByRole("button", { name: "Log food" }).click();
  await expect(page.getByLabel("Food energy total")).toHaveText("406 kcal");

  await page.getByRole("button", { name: "log out" }).click();
  await page.getByPlaceholder("new-user-sub").fill(`food-private-other-${Date.now()}`);
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await chooseTimezone(page);
  await expect(page.getByLabel("Food energy total")).toHaveText("0 kcal");
  await expect(page.getByText("SOKERI", { exact: true })).toHaveCount(0);
});

test("food search, preview, and logging work without JavaScript", async ({ browser }, testInfo) => {
  const context = await browser.newContext({
    baseURL: "http://127.0.0.1:8200",
    javaScriptEnabled: false,
  });
  const page = await context.newPage();

  await login(page, testInfo);
  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await page.getByRole("link", { name: /SOKERI/ }).click();
  await page.getByRole("spinbutton", { name: "Amount in grams" }).fill("50");
  await page.getByRole("button", { name: "Preview", exact: true }).click();

  await expect(page.getByText("203 kcal", { exact: true })).toBeVisible();
  await page.getByLabel("New meal name (optional)").fill("Breakfast");
  await page.getByRole("button", { name: "Log food" }).click();
  await expect(page.getByLabel("Food energy total")).toHaveText("203 kcal");
  await expect(page.getByRole("heading", { name: /^Breakfast ·/ })).toBeVisible();
  await context.close();
});
