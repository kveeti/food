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
  const amount = page.getByRole("spinbutton", { name: "Amount in grams" });
  await amount.fill("");
  await expect(amount).toBeFocused();
  await expect(page.getByText("Nutrition per 100 g", { exact: true })).toBeVisible();
  await expect(page.locator("#food-nutrition").getByText("406 kcal", { exact: true })).toBeVisible();
  const energyDetail = page.locator('[data-nutrient-detail][data-unit="kJ"]');
  await expect(energyDetail).toHaveText("1698 kJ");
  await amount.fill("50");
  await expect(amount).toBeFocused();
  await expect(page.getByText("Nutrition for 50 g", { exact: true })).toBeVisible();
  await expect(page.getByText("203 kcal", { exact: true })).toBeVisible();
  await expect(energyDetail).toHaveText("849 kJ");
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
  const saveRequestPromise = page.waitForRequest((request) =>
    new URL(request.url()).pathname.startsWith("/food-entries/"),
  );
  await page.getByRole("button", { name: "Save", exact: true }).click();
  const saveRequest = await saveRequestPromise;
  expect(saveRequest.headers()["hx-request"]).toBe("true");
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
  let deleteDialog = page.getByRole("dialog", { name: "Delete food?" });
  await expect(deleteDialog).toBeVisible();
  await deleteDialog.getByRole("button", { name: "No, cancel" }).click();
  await expect(deleteDialog).not.toBeVisible();
  await page.getByRole("button", { name: "Delete SOKERI" }).last().click();
  await expect(deleteDialog).toBeVisible();
  const deleteRequestPromise = page.waitForRequest((request) =>
    new URL(request.url()).pathname.endsWith("/delete"),
  );
  await deleteDialog.getByRole("button", { name: "Yes, delete" }).click();
  const deleteRequest = await deleteRequestPromise;
  expect(deleteRequest.headers()["hx-request"]).toBe("true");
  await expect(page.getByLabel("Food energy total")).toHaveText("406 kcal");
  await page.getByText("SOKERI", { exact: true }).click();
  await page.getByRole("button", { name: "Delete SOKERI" }).click();
  deleteDialog = page.getByRole("dialog", { name: "Delete food?" });
  await deleteDialog.getByRole("button", { name: "Yes, delete" }).click();
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

  const copyLunch = page.getByRole("link", { name: "Copy Lunch meal" });
  await expect(copyLunch).toBeVisible();
  await expect(page.getByRole("link", { name: "Copy Snack meal" })).toBeVisible();
  await copyLunch.click();
  await expect(page).toHaveURL(/meal_preview=/);
  const copiedMeal = page.locator("#food-preview");
  await expect(copiedMeal.getByRole("heading", { name: "New meal" })).toBeVisible();
  await expect(copiedMeal.getByLabel("Meal name")).toHaveValue("Lunch");
  await expect(copiedMeal).toBeInViewport();
  const copiedAmount = copiedMeal.getByRole("spinbutton", {
    name: "Amount for SOKERI in grams",
  });
  await expect(copiedAmount).toHaveValue("100");
  await copiedAmount.fill("75");
  await expect(copiedAmount).toHaveValue("75");
  const includeSugar = copiedMeal.getByRole("checkbox", { name: "SOKERI" });
  const selectAll = copiedMeal.getByRole("checkbox", { name: "Select all" });
  await expect(selectAll).toBeChecked();
  await selectAll.uncheck();
  await expect(includeSugar).not.toBeChecked();
  await expect(copiedAmount).toBeDisabled();
  await selectAll.check();
  await expect(includeSugar).toBeChecked();
  await expect(copiedAmount).toBeEnabled();
  await includeSugar.uncheck();
  await expect(selectAll).not.toBeChecked();
  await expect(copiedAmount).toBeDisabled();
  await selectAll.check();
  await expect(copiedAmount).toBeEnabled();
  const cancelRequest = page.waitForRequest((request) => {
    const url = new URL(request.url());
    return url.pathname === "/" && !url.searchParams.has("meal_preview");
  });
  await copiedMeal.getByRole("link", { name: "Cancel" }).click();
  expect((await cancelRequest).headers()["hx-request"]).toBe("true");
  await expect(page).not.toHaveURL(/meal_preview=/);
  await expect(page.locator("#food-preview")).toBeEmpty();

  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  await page.getByRole("link", { name: /SOKERI/ }).click();
  const meal = page.locator('select[name="meal"]');
  await expect(meal.locator("option")).toContainText([
    "Start a new meal",
    /Continue Snack ·/,
  ]);
  await expect(meal).not.toContainText("Continue Lunch");

  const search = page.getByRole("searchbox", { name: "Search foods" });
  await search.fill("Lunch");
  const lunch = page.locator('#food-results a[data-result-type="meal"]', {
    hasText: "Lunch",
  });
  await expect(lunch).toHaveCount(1);

  await search.fill("SOKERI");
  await expect(page.locator('#food-results a[data-result-type="meal"]')).toHaveCount(1);
  await lunch.click();
  const preview = page.locator("#food-preview");
  await expect(preview.getByRole("heading", { name: "New meal" })).toBeVisible();
  const copiedMealName = preview.getByLabel("Meal name");
  await expect(copiedMealName).toHaveValue("Lunch");
  await copiedMealName.fill("Second lunch");
  await expect(preview.getByText("SOKERI", { exact: true })).toBeVisible();
  await expect(preview.getByRole("link", { name: "Cancel" })).toBeVisible();
  await preview.getByRole("spinbutton", { name: "Amount for SOKERI in grams" }).fill("75");
  await preview.getByRole("button", { name: "Add meal" }).click();
  await expect(page.getByLabel("Food energy total")).toHaveText("754 kcal");
  await expect(page.getByRole("heading", { name: /^Lunch ·/ })).toHaveCount(1);
  await expect(page.getByRole("heading", { name: /^Second lunch ·/ })).toBeVisible();
  await expect(page.getByText("SOKERI", { exact: true })).toHaveCount(2);
  await expect(page.getByText(/^Continuing Second lunch ·/)).toBeVisible();
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
  await expect(page.getByText("Nutrition per 100 g", { exact: true })).toBeVisible();
  await expect(page.locator("#food-nutrition").getByText("406 kcal", { exact: true })).toBeVisible();
  await page.getByLabel("New meal name (optional)").fill("Breakfast");
  await page.getByRole("button", { name: "Log food" }).click();
  await expect(page.getByLabel("Food energy total")).toHaveText("203 kcal");
  await expect(page.getByRole("heading", { name: /^Breakfast ·/ })).toBeVisible();
  await page.getByRole("link", { name: "Copy Breakfast meal" }).click();
  const mealPreview = page.locator("#food-preview");
  await expect(mealPreview.getByRole("heading", { name: "New meal" })).toBeVisible();
  await expect(mealPreview.getByLabel("Meal name")).toHaveValue("Breakfast");
  await expect(mealPreview.getByRole("spinbutton", { name: "Amount for SOKERI in grams" })).toHaveValue("50");
  await expect(mealPreview.getByRole("checkbox", { name: "Select all" })).toHaveCount(0);
  await expect(mealPreview.getByRole("link", { name: "Cancel" })).toBeVisible();
  await mealPreview.getByRole("button", { name: "Add meal" }).click();
  await expect(page.getByLabel("Food energy total")).toHaveText("406 kcal");
  await expect(page.getByRole("heading", { name: /^Breakfast ·/ })).toHaveCount(2);

  await page.getByText("SOKERI", { exact: true }).first().click();
  await page.getByRole("button", { name: "Delete SOKERI" }).click();
  const deleteDialog = page.getByRole("dialog", { name: "Delete food?" });
  await expect(deleteDialog).toHaveCSS("opacity", "1");
  await deleteDialog.getByRole("button", { name: "No, cancel" }).click();
  await expect(deleteDialog).not.toBeVisible();
  await page.getByRole("button", { name: "Delete SOKERI" }).click();
  await expect(deleteDialog).toHaveCSS("opacity", "1");
  await deleteDialog.getByRole("button", { name: "Yes, delete" }).click();
  await expect(page.getByText("SOKERI", { exact: true })).toHaveCount(1);

  await context.close();
});
