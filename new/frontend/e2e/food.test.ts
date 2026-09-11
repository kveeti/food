import { test, expect } from "./fixtures.ts";
import { finishSetup, login } from "./helpers.ts";

test("keeps the previous empty result visible during the next search", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  const search = page.getByRole("combobox", { name: "Search foods" });
  const empty = page.getByText("No foods found.", { exact: true });
  await search.fill("zzzz");
  await expect(empty).toBeVisible();

  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/foods?q=zzzzz", async (route) => {
    await gate;
    await route.continue();
  });
  try {
    await search.press("z");
    await expect(search).toHaveAttribute("aria-busy", "true");
    await expect(empty).toBeVisible();
  } finally {
    release();
  }
  await expect(search).toHaveAttribute("aria-busy", "false");
  await expect(empty).toBeVisible();
  await search.fill("apple");
  await expect(
    page.getByRole("option", { name: "Apple Fineli" }),
  ).toBeVisible();
  await expect(empty).toHaveCount(0);
});

test("counts Unicode search characters instead of bytes", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  const query = "ä".repeat(101);
  const searched = page.waitForResponse(
    (response) =>
      new URL(response.url()).searchParams.get("q") === query &&
      response.request().method() === "GET",
  );
  await page.getByRole("combobox", { name: "Search foods" }).fill(query);
  expect((await searched).ok()).toBe(true);
  await expect(
    page.getByText("No foods found.", { exact: true }),
  ).toBeVisible();
});

test("keeps previous search results usable while the next search loads", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  const search = page.getByRole("combobox", { name: "Search foods" });
  await search.fill("apple");
  const apple = page.getByRole("option", { name: "Apple Fineli" });
  await expect(apple).toBeVisible();

  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/foods?q=orch", async (route) => {
    await gate;
    await route.continue();
  });
  try {
    await search.fill("orch");
    await expect(search).toHaveAttribute("aria-busy", "true");
    await expect(apple).toBeVisible();
    await expect(apple).toBeEnabled();
    await expect(
      page.getByText("No foods found.", { exact: true }),
    ).toHaveCount(0);
  } finally {
    release();
  }
  await expect(search).toHaveAttribute("aria-busy", "false");
  await expect(apple).toHaveCount(0);
  await expect(
    page.getByRole("option", { name: "Apple juice Orchard · Open Food Facts" }),
  ).toBeEnabled();
  await search.fill("zzzzzz");
  await expect(
    page.getByText("No foods found.", { exact: true }),
  ).toBeVisible();
  await search.fill("apple");
  await expect(apple).toBeVisible();
  await expect(page.getByText("No foods found.", { exact: true })).toHaveCount(
    0,
  );
});

test("retries failed meal loads", async ({ page }) => {
  await login(page);
  let failing = true;
  await page.route("**/api/meals?*", (route) =>
    failing
      ? route.fulfill({ status: 500, body: "Meal read failed" })
      : route.continue(),
  );
  await finishSetup(page);

  const food = page.getByRole("region", { name: "Food", exact: true });
  await expect(food.getByRole("alert")).toHaveText("Error loading meals", {
    timeout: 15_000,
  });
  failing = false;
  await food.getByRole("button", { name: "Try again" }).click();
  await expect(food.getByText("No meals logged for this day")).toBeVisible();
  await expect(food.getByRole("alert")).toHaveCount(0);
});

test("retries failed food searches", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  let failing = true;
  await page.route("**/api/foods?q=apple", (route) =>
    failing
      ? route.fulfill({ status: 500, body: "Search failed" })
      : route.continue(),
  );

  await page.getByRole("combobox", { name: "Search foods" }).fill("apple");
  const error = page
    .getByRole("alert")
    .filter({ hasText: "Error searching foods" });
  await expect(error).toBeVisible({ timeout: 15_000 });
  failing = false;
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("option", { name: "Apple Fineli" }),
  ).toBeVisible();
  await expect(error).toHaveCount(0);
});

test("retries failed food detail loads", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  const search = page.getByRole("combobox", { name: "Search foods" });
  await search.fill("apple");
  const apple = page.getByRole("option", { name: "Apple Fineli" });
  await expect(apple).toBeVisible();

  let failing = true;
  await page.route("**/api/foods/*", (route) =>
    failing
      ? route.fulfill({ status: 500, body: "Food read failed" })
      : route.continue(),
  );
  await apple.click();

  const newFood = page.getByRole("region", { name: "New food" });
  await expect(newFood.getByRole("alert")).toHaveText("Error loading food", {
    timeout: 15_000,
  });
  failing = false;
  await newFood.getByRole("button", { name: "Try again" }).click();
  await expect(newFood.getByLabel("Amount (g)")).toBeVisible();
  await expect(newFood.getByRole("alert")).toHaveCount(0);
});

test("searches, loads a skeleton, and logs and deletes food with keyboard", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  const searchSection = page.getByRole("region", {
    name: "Search",
    exact: true,
  });
  const foodSection = page.getByRole("region", { name: "Food", exact: true });
  const search = searchSection.getByRole("combobox", { name: "Search foods" });
  await expect(
    searchSection.getByPlaceholder("Search foods, brands, meals..."),
  ).toBeVisible();
  await expect(foodSection.getByRole("combobox")).toHaveCount(0);
  await search.pressSequentially("omena");
  await expect(
    page.getByRole("option", { name: "Apple Fineli" }),
  ).toBeVisible();

  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/foods/*", async (route) => {
    await gate;
    await route.continue();
  });
  const newFood = page.getByRole("region", { name: "New food", exact: true });
  try {
    await search.press("Enter");
    await expect(page).toHaveURL(
      (url) =>
        Boolean(url.searchParams.get("food")) && !url.searchParams.has("q"),
    );
    await expect(
      newFood.getByRole("status", { name: "Loading food details" }),
    ).toBeVisible();
    await expect(newFood.getByLabel("Amount", { exact: true })).toBeFocused();
    await expect(
      newFood.getByText("Selected food", { exact: true }),
    ).toHaveCount(0);
    await expect(searchSection.getByLabel("Amount (g)")).toHaveCount(0);
    const selectedUrl = page.url();
    await page.reload();
    await expect(page).toHaveURL(selectedUrl);
    await expect(
      newFood.getByRole("status", { name: "Loading food details" }),
    ).toBeVisible();
    const pendingAmount = newFood.getByLabel("Amount", { exact: true });
    await expect(pendingAmount).toBeFocused();
    await pendingAmount.fill("123");
    await expect(
      newFood.getByRole("button", { name: "Add food", exact: true }),
    ).toBeDisabled();
  } finally {
    release();
  }
  const amount = newFood.getByLabel("Amount (g)");
  await expect(amount).toBeFocused();
  await expect(amount).toHaveValue("123");
  await amount.fill("");
  await page.getByRole("button", { name: "Add food", exact: true }).click();
  await expect(
    page.getByText("Enter an amount", { exact: true }),
  ).toBeVisible();
  await amount.fill("0");
  await amount.press("Enter");
  await expect(
    page.getByText("Amount must be greater than 0", { exact: true }),
  ).toBeVisible();
  await amount.fill("150,5");
  await newFood
    .getByRole("combobox", { name: "Meal", exact: true })
    .selectOption("breakfast");
  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/food-entries") &&
      response.request().method() === "POST",
  );
  await amount.press("Enter");
  const savedResponse = await saved;
  expect(savedResponse.ok()).toBe(true);
  expect(savedResponse.request().postDataJSON()).not.toHaveProperty("id");
  expect(savedResponse.request().postDataJSON()).not.toHaveProperty(
    "renderKey",
  );
  await expect(search).not.toBeFocused();
  const entries = foodSection.getByRole("list", { name: "Breakfast foods" });
  await expect(entries).toContainText("150.5 g");
  await expect(entries).toContainText("75 kcal");
  await page.reload();
  await expect(entries).toContainText("Apple");
  await expect(entries).toContainText("75 kcal");
  const deleted = page.waitForResponse(
    (response) => response.request().method() === "DELETE",
  );
  await page.getByRole("button", { name: "Edit Apple entry" }).click();
  await page.getByRole("button", { name: "Delete Apple entry" }).click();
  expect((await deleted).ok()).toBe(true);
  await page.reload();
  await expect(page.getByText("No meals logged for this day")).toBeVisible();
});

test("logs volume on a selected day and rolls back failed writes", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.goto("/?date=2026-01-10");
  const search = page.getByRole("combobox", { name: "Search foods" });
  await search.fill("orch");
  await page
    .getByRole("option", { name: "Apple juice Orchard · Open Food Facts" })
    .click();
  const amount = page.getByLabel("Amount (ml)");
  await amount.fill("250");
  await page
    .getByRole("combobox", { name: "Meal", exact: true })
    .selectOption("lunch");
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/food-entries", async (route) => {
    await gate;
    await route.fulfill({ status: 500, body: "Could not save food" });
  });
  await amount.press("Enter");
  const entries = page.getByRole("list", { name: "Lunch foods" });
  await expect(entries).toContainText("250 ml");
  await expect(page.getByRole("region", { name: "New food" })).toHaveCount(0);
  await expect(search).not.toBeFocused();
  release();
  await expect(page.getByRole("alert")).toHaveText("Error adding food");
  await expect(entries.getByRole("listitem")).toHaveCount(0);
  await expect(amount).toHaveValue("250");
  await page.unroute("**/api/food-entries");
  const saved = page.waitForResponse(
    (response) => response.request().method() === "POST",
  );
  await amount.press("Enter");
  const response = await saved;
  expect(response.ok()).toBe(true);
  await expect(search).not.toBeFocused();
  await page.reload();
  await expect(entries).toContainText("250 ml");
  await page.getByRole("link", { name: "Next day" }).click();
  await expect(page.getByText("No meals logged for this day")).toBeVisible();
  await page.getByRole("link", { name: "Previous day" }).click();
  await expect(entries).toContainText("250 ml");
});

test("continues meals and edits, cancels, retries and deletes inline", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  const search = page.getByRole("combobox", { name: "Search foods" });
  const newFood = page.getByRole("region", { name: "New food" });
  await search.fill("omena");
  await page.getByRole("option", { name: "Apple Fineli" }).click();
  await expect(
    newFood.getByRole("combobox", { name: "Meal", exact: true }),
  ).toHaveValue("");
  await newFood.getByLabel("Amount (g)").fill("100");
  await newFood.getByRole("button", { name: "Add food", exact: true }).click();
  await expect(
    newFood.getByText("Choose a meal", { exact: true }),
  ).toBeVisible();
  await newFood
    .getByRole("combobox", { name: "Meal", exact: true })
    .selectOption("lunch");
  await newFood.getByRole("button", { name: "Add food", exact: true }).click();
  await expect(newFood).toHaveCount(0);
  await search.fill("orch");
  await page
    .getByRole("option", { name: "Apple juice Orchard · Open Food Facts" })
    .click();
  await expect(
    newFood.getByRole("combobox", { name: "Meal", exact: true }),
  ).toHaveValue("continue_previous");
  await newFood.getByLabel("Amount (ml)").fill("200");
  const added = page.waitForResponse(
    (response) => response.request().method() === "POST",
  );
  await newFood.getByRole("button", { name: "Add food", exact: true }).click();
  await expect(newFood).toHaveCount(0);
  const addedResponse = await added;
  expect(addedResponse.ok()).toBe(true);
  expect(addedResponse.request().postDataJSON().meal_id).toBeTruthy();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Lunch", exact: true }),
  ).toHaveCount(1);
  const foods = page.getByRole("list", { name: "Lunch foods" });
  await expect(foods.getByRole("listitem")).toHaveCount(2);
  const edit = foods.getByRole("button", {
    name: "Edit Apple entry",
    exact: true,
  });
  const appleRow = edit.locator("..");
  await edit.click();
  await foods.getByLabel("Amount (g)").fill("150,5");
  await expect(foods.getByText("75 kcal", { exact: true })).toBeVisible();
  await foods.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(appleRow).toContainText("100 g");
  await edit.click();
  await foods.getByLabel("Amount (g)").fill("200");
  await page.route("**/api/food-entries/*", async (route) => {
    if (route.request().method() === "PATCH")
      await route.fulfill({ status: 500, body: "Could not update food" });
    else await route.continue();
  });
  await foods.getByRole("button", { name: "Save", exact: true }).click();
  await expect(foods.getByRole("alert")).toHaveText("Error updating food");
  await expect(foods.getByLabel("Amount (g)")).toHaveValue("200");
  await page.unroute("**/api/food-entries/*");
  await foods.getByRole("button", { name: "Save", exact: true }).click();
  await expect(appleRow).toContainText("200 g");
  await page.reload();
  await expect(appleRow).toContainText("200 g");
  await expect(appleRow).toContainText("100 kcal");
  await edit.click();
  let releaseDelete!: () => void;
  const deleteGate = new Promise<void>((resolve) => {
    releaseDelete = resolve;
  });
  await page.route("**/api/food-entries/*", async (route) => {
    if (route.request().method() === "DELETE") {
      await deleteGate;
      await route.fulfill({ status: 500, body: "Could not delete food" });
    } else await route.continue();
  });
  await foods
    .getByRole("button", { name: "Delete Apple entry", exact: true })
    .click();
  await expect(foods.locator("li[aria-hidden=true]")).toHaveCount(1);
  await expect(foods).toContainText("Apple juice");
  releaseDelete();
  await expect(foods.getByRole("alert")).toHaveText("Error deleting food");
  await expect(foods.getByLabel("Amount (g)")).toHaveValue("200");
  await page.unroute("**/api/food-entries/*");
  await foods
    .getByRole("button", { name: "Delete Apple entry", exact: true })
    .click();
  await expect(foods.getByLabel("Amount (g)")).toHaveCount(0);
  await page.reload();
  await expect(foods.getByRole("listitem")).toHaveCount(1);

  // A pending deletion hides the row but keeps its local error state mounted.
  const lastEdit = foods.getByRole("button", {
    name: "Edit Apple juice entry",
    exact: true,
  });
  await lastEdit.click();
  const lastDeleteGate = new Promise<void>((resolve) => {
    releaseDelete = resolve;
  });
  await page.route("**/api/food-entries/*", async (route) => {
    if (route.request().method() === "DELETE") {
      await lastDeleteGate;
      await route.fulfill({ status: 500, body: "Could not delete food" });
    } else await route.continue();
  });
  const lastMeal = page.locator("article").filter({ hasText: "Apple juice" });
  await foods.getByRole("button", { name: "Delete Apple juice entry" }).click();
  await expect(lastMeal.locator("li[aria-hidden=true]")).toHaveCount(1);
  await expect(lastMeal).toHaveAttribute("aria-hidden", "true");
  await expect(page.getByText("No meals logged for this day")).toBeVisible();
  releaseDelete();
  await expect(foods.getByRole("alert")).toHaveText("Error deleting food");
  await expect(lastMeal).not.toHaveAttribute("aria-hidden", "true");
  await expect(page.getByText("No meals logged for this day")).toHaveCount(0);
  await expect(foods.getByLabel("Amount (ml)")).toHaveValue("200");
  await page.unroute("**/api/food-entries/*");
  let releaseSuccessfulDelete!: () => void;
  const successfulDeleteGate = new Promise<void>((resolve) => {
    releaseSuccessfulDelete = resolve;
  });
  await page.route("**/api/food-entries/*", async (route) => {
    if (route.request().method() === "DELETE") {
      await successfulDeleteGate;
    }
    await route.continue();
  });
  const deleted = page.waitForResponse(
    (response) => response.request().method() === "DELETE",
  );
  await foods.getByRole("button", { name: "Delete Apple juice entry" }).click();
  await expect(page.getByText("No meals logged for this day")).toBeVisible();
  await expect
    .poll(() =>
      lastMeal.evaluate((element) => element.getBoundingClientRect().height),
    )
    .toBe(0);
  const empty = page.getByText("No meals logged for this day");
  const pendingEmptyTop = await empty.evaluate(
    (element) => element.getBoundingClientRect().top,
  );
  releaseSuccessfulDelete();
  expect((await deleted).ok()).toBe(true);
  await expect(lastMeal).toHaveCount(0);
  expect(
    await empty.evaluate((element) => element.getBoundingClientRect().top),
  ).toBe(pendingEmptyTop);
  await expect(foods).toHaveCount(0);
  await page.reload();
  await expect(page.getByText("No meals logged for this day")).toBeVisible();
});
