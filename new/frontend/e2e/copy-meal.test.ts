import type { Page } from "@playwright/test";

import type { FoodEntry, FoodMeal } from "../src/api/food.ts";
import { test, expect } from "./fixtures.ts";
import { finishSetup, login } from "./helpers.ts";

async function addFood(
  page: Page,
  date: string,
  meal: string,
  mealId?: string,
  isJuice = false,
) {
  const response = await page.request.post("/api/food-entries", {
    data: {
      date,
      meal,
      meal_id: mealId ?? null,
      food_id: isJuice
        ? "00000000-0000-4000-8000-000000000002"
        : "00000000-0000-4000-8000-000000000001",
      amount: isJuice ? 200 : 100,
      unit: isJuice ? "ml" : "g",
    },
  });
  expect(response.ok()).toBe(true);
  return response.json() as Promise<FoodEntry>;
}

async function mealsFor(page: Page, date: string) {
  const response = await page.request.get(`/api/meals?date=${date}`);
  expect(response.ok()).toBe(true);
  return response.json() as Promise<FoodMeal[]>;
}

async function setup(page: Page) {
  await login(page);
  await finishSetup(page);
  const today = (await page
    .locator('[aria-current="date"]')
    .getAttribute("data-date"))!;
  const date = new Date(`${today}T12:00:00Z`);
  date.setUTCDate(date.getUTCDate() - 1);
  const yesterday = date.toISOString().slice(0, 10);
  const apple = await addFood(page, yesterday, "breakfast");
  const juice = await addFood(
    page,
    yesterday,
    "continue_previous",
    apple.meal_id!,
    true,
  );
  return { today, yesterday, apple, juice };
}

for (const isDesktop of [false, true]) {
  test(`copies edited foods without leaving the source position on ${isDesktop ? "desktop" : "mobile"}`, async ({
    page,
  }) => {
    if (isDesktop) {
      await page.setViewportSize({ width: 1280, height: 900 });
      await page.emulateMedia({ colorScheme: "dark" });
    }
    const { today, yesterday } = await setup(page);
    await Promise.all(
      Array.from({ length: 8 }, () => addFood(page, yesterday, "dinner")),
    );
    await page.goto(`/?date=${yesterday}`);
    const source = page.getByRole("article", {
      name: "Breakfast",
      exact: true,
    });
    await source.getByRole("button", { name: "Open Breakfast meal" }).click();
    const trigger = source.getByRole("button", { name: "Copy", exact: true });
    await trigger.scrollIntoViewIfNeeded();
    const scroller = isDesktop
      ? page.getByRole("main")
      : page.getByRole("region", { name: "Food", exact: true });
    await expect
      .poll(() => scroller.evaluate((element) => element.scrollTop))
      .toBeGreaterThan(0);
    const sourceScroll = await scroller.evaluate(
      (element) => element.scrollTop,
    );
    await trigger.click();
    const dialog = page.getByRole("dialog", { name: "Copy meal", exact: true });
    await expect(dialog).toBeVisible();
    const box = await dialog.boundingBox();
    expect(box!.width).toBeLessThanOrEqual(isDesktop ? 449 : 391);
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.y).toBeGreaterThanOrEqual(0);
    await expect(dialog.getByLabel("Date", { exact: true })).toHaveValue(today);
    await expect(
      dialog.getByRole("tab", { name: "New meal", exact: true }),
    ).toHaveAttribute("aria-selected", "true");
    await expect(dialog.getByLabel("Meal", { exact: true })).toHaveValue(
      "breakfast",
    );
    await expect(
      dialog.getByRole("tab", { name: "Existing meal" }),
    ).toBeDisabled();
    await dialog
      .getByRole("checkbox", { name: "Apple juice Orchard" })
      .uncheck();
    await dialog.getByLabel("Apple amount (g)").fill("125,5");
    await dialog.getByLabel("Time", { exact: true }).fill("06:45");
    await dialog
      .getByRole("button", { name: "Copy meal", exact: true })
      .click();
    await expect(dialog).not.toBeVisible();
    await expect(page.getByText("Meal copied", { exact: true })).toBeVisible();
    const notification = page
      .locator("[data-sonner-toast]")
      .filter({ hasText: "Meal copied" });
    await notification.hover({ position: { x: 10, y: 10 } });
    const appFont = await page
      .locator("body")
      .evaluate((element) => getComputedStyle(element).fontFamily);
    expect(
      await notification.evaluate(
        (element) => getComputedStyle(element).fontFamily,
      ),
    ).toBe(appFont);
    const view = notification.getByRole("button", {
      name: "View",
      exact: true,
    });
    expect(
      await view.evaluate(
        (element) => getComputedStyle(element).backgroundColor,
      ),
    ).toBe("rgba(0, 0, 0, 0)");
    await view.hover();
    expect(
      await view.evaluate(
        (element) => getComputedStyle(element).backgroundColor,
      ),
    ).not.toBe("rgba(0, 0, 0, 0)");
    await expect(page).toHaveURL(new RegExp(`date=${yesterday}$`));
    await expect
      .poll(() => scroller.evaluate((element) => element.scrollTop))
      .toBeCloseTo(sourceScroll, 0);
    await expect(source.getByRole("listitem")).toHaveCount(2);
    const copies = await mealsFor(page, today);
    expect(copies).toHaveLength(1);
    expect(copies[0]!.entries).toHaveLength(1);
    expect(copies[0]!.entries[0]!.amount).toBe(125.5);
    expect(copies[0]!.entries[0]!.food_name).toBe("Apple");
    const time = new Intl.DateTimeFormat("en-GB", {
      timeZone: "Europe/Helsinki",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(copies[0]!.started_at));
    expect(time).toBe("06:45");
    await page.getByRole("button", { name: "View", exact: true }).click();
    await expect(page).toHaveURL(
      new RegExp(`date=${today}&meal=${copies[0]!.id}$`),
    );
    await expect(page.locator(`#food-meal-${copies[0]!.id}`)).toBeInViewport();
  });

  test(`keeps the source in view when copying onto the same day on ${isDesktop ? "desktop" : "mobile"}`, async ({
    page,
  }) => {
    if (isDesktop) await page.setViewportSize({ width: 1280, height: 900 });
    const { yesterday, apple } = await setup(page);
    await Promise.all(
      Array.from({ length: 8 }, () => addFood(page, yesterday, "dinner")),
    );
    await page.goto(`/?date=${yesterday}`);
    const source = page.locator(`#food-meal-${apple.meal_id}`);
    await source.getByRole("button", { name: "Open Breakfast meal" }).click();
    const trigger = source.getByRole("button", { name: "Copy", exact: true });
    await trigger.scrollIntoViewIfNeeded();
    const position = (await source.boundingBox())!.y;
    await trigger.click();
    const dialog = page.getByRole("dialog", { name: "Copy meal", exact: true });
    await dialog.getByLabel("Date", { exact: true }).fill(yesterday);
    await dialog.getByLabel("Time", { exact: true }).fill("23:59");
    await dialog
      .getByRole("button", { name: "Copy meal", exact: true })
      .click();
    await expect(dialog).not.toBeVisible();
    await expect(
      page.getByText("Meal copied", { exact: true }),
    ).not.toBeVisible();
    await expect(page).toHaveURL(new RegExp(`date=${yesterday}$`));
    await expect(page.getByRole("article")).toHaveCount(10);
    await expect
      .poll(async () => (await source.boundingBox())!.y)
      .toBeCloseTo(position, 0);
    await expect(source.getByRole("listitem")).toHaveCount(2);
  });

  test(`expands meal actions and restores a failed deletion on ${isDesktop ? "desktop" : "mobile"}`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    if (isDesktop) await page.setViewportSize({ width: 1280, height: 900 });
    const { yesterday, apple } = await setup(page);
    await page.goto(`/?date=${yesterday}`);
    const source = page.getByRole("article", {
      name: "Breakfast",
      exact: true,
    });
    const heading = source.getByRole("button", {
      name: /^(Open|Close) Breakfast meal$/,
    });
    await expect(heading).toHaveAttribute("aria-expanded", "false");
    await expect(
      source.getByRole("button", { name: "Copy", exact: true }),
    ).not.toBeVisible();
    await expect(
      source.getByRole("button", { name: "Delete Breakfast meal" }),
    ).not.toBeVisible();
    await heading.focus();
    await heading.press("Enter");
    await expect(heading).toHaveAttribute("aria-expanded", "true");
    await expect(
      source.getByRole("button", { name: "Copy", exact: true }),
    ).toBeVisible();
    await expect(source.getByRole("listitem")).toHaveCount(2);
    await heading.press("Space");
    await expect(heading).toHaveAttribute("aria-expanded", "false");
    await expect(
      source.getByRole("button", { name: "Copy", exact: true }),
    ).not.toBeVisible();
    await expect(source.getByRole("listitem")).toHaveCount(2);
    await heading.click();

    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    let isFailing = true;
    let deleteRequests = 0;
    await page.route(`**/api/meals/${apple.meal_id}`, async (route) => {
      deleteRequests += 1;
      if (!isFailing) return route.continue();
      await gate;
      await route.fulfill({ status: 500, body: "Delete failed" });
    });
    const deleteButton = source.getByRole("button", {
      name: "Delete Breakfast meal",
    });
    const confirmation = page.getByRole("alertdialog", {
      name: "Delete Breakfast?",
      exact: true,
    });
    await deleteButton.click();
    await expect(confirmation).toBeVisible();
    const dialogBackground = await confirmation.evaluate(
      (element) => getComputedStyle(element, "::before").backgroundColor,
    );
    const pageBackground = await page
      .locator("body")
      .evaluate((element) => getComputedStyle(element).backgroundColor);
    expect(dialogBackground).not.toBe(pageBackground);
    await expect(
      confirmation.getByRole("button", { name: "No, cancel" }),
    ).toBeFocused();
    await confirmation.getByRole("button", { name: "No, cancel" }).click();
    await expect(confirmation).not.toBeVisible();
    await expect(deleteButton).toBeFocused();
    await expect(source.getByRole("listitem")).toHaveCount(2);
    expect(deleteRequests).toBe(0);
    await deleteButton.click();
    await expect(confirmation).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(confirmation).not.toBeVisible();
    expect(deleteRequests).toBe(0);
    try {
      await deleteButton.click();
      await confirmation.getByRole("button", { name: "Yes, delete" }).click();
      await expect(confirmation).not.toBeVisible();
      await expect(source).toHaveCount(0);
      await expect(
        page.getByText("No meals logged for this day"),
      ).toBeVisible();
    } finally {
      release();
    }
    await expect(source.getByRole("alert")).toHaveText("Error deleting meal");
    await expect(heading).toHaveAttribute("aria-expanded", "true");
    await expect(source.getByRole("listitem")).toHaveCount(2);
    expect((await mealsFor(page, yesterday))[0]!.entries).toHaveLength(2);
    isFailing = false;
    const deleted = page.waitForResponse(
      (response) =>
        response.request().method() === "DELETE" &&
        response.url().endsWith(`/api/meals/${apple.meal_id}`),
    );
    await deleteButton.click();
    await confirmation.getByRole("button", { name: "Yes, delete" }).click();
    expect((await deleted).ok()).toBe(true);
    expect(deleteRequests).toBe(2);
    await expect(source).toHaveCount(0);
    await expect(page.getByText("No meals logged for this day")).toBeVisible();
    expect(await mealsFor(page, yesterday)).toHaveLength(0);
    await expect(
      page.getByLabel("Calorie total", { exact: true }),
    ).toContainText("0");
    await page.reload();
    await expect(page.getByText("No meals logged for this day")).toBeVisible();
  });
}

test("selects an older existing meal and clears it when the target date changes", async ({
  page,
}) => {
  const { today, yesterday, apple } = await setup(page);
  const lunch = await addFood(page, today, "lunch");
  const dinner = await addFood(page, today, "dinner");
  await page.goto(`/?date=${yesterday}`);
  const source = page.getByRole("article", { name: "Breakfast", exact: true });
  await source.getByRole("button", { name: "Open Breakfast meal" }).click();
  await source.getByRole("button", { name: "Copy", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Copy meal", exact: true });
  await dialog.getByRole("tab", { name: "Existing meal" }).click();
  const select = dialog.getByLabel("Meal", { exact: true });
  await expect(select).toHaveValue("continue_previous");
  await expect(
    select.locator('option[value="continue_previous"]'),
  ).toContainText("Dinner");
  await select.selectOption(lunch.meal_id!);
  await expect(dialog.getByLabel("Time", { exact: true })).toHaveCount(0);
  await dialog.getByLabel("Apple juice amount (ml)").fill("75");
  await dialog.getByLabel("Date", { exact: true }).fill(yesterday);
  await dialog.getByRole("tab", { name: "Existing meal" }).click();
  await expect(select).toHaveValue("continue_previous");
  await expect(
    select.locator('option[value="continue_previous"]'),
  ).toContainText("Breakfast");
  await expect(select.locator(`option[value="${lunch.meal_id}"]`)).toHaveCount(
    0,
  );
  await expect(dialog.getByLabel("Apple juice amount (ml)")).toHaveValue("75");
  await dialog.getByLabel("Date", { exact: true }).fill(today);
  await dialog.getByRole("tab", { name: "Existing meal" }).click();
  await select.selectOption(lunch.meal_id!);
  await dialog.getByRole("button", { name: "Copy meal", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const meals = await mealsFor(page, today);
  expect(meals).toHaveLength(2);
  const copied = meals.find((meal) => meal.id === lunch.meal_id)!;
  expect(copied.entries).toHaveLength(3);
  expect(
    copied.entries.every((entry) => entry.eaten_at === lunch.eaten_at),
  ).toBe(true);
  expect(
    copied.entries.find((entry) => entry.food_name === "Apple juice")!.amount,
  ).toBe(75);
  expect(
    meals.find((meal) => meal.id === dinner.meal_id)!.entries,
  ).toHaveLength(1);
  const original = (await mealsFor(page, yesterday)).find(
    (meal) => meal.id === apple.meal_id,
  )!;
  expect(original.entries).toHaveLength(2);
});

for (const isDesktop of [false, true]) {
  test(`blocks edits and dismissal while copying and restores failed edits on ${isDesktop ? "desktop" : "mobile"}`, async ({
    page,
  }) => {
    if (isDesktop) await page.setViewportSize({ width: 900, height: 480 });
    const { today, yesterday } = await setup(page);
    await page.goto(`/?date=${yesterday}`);
    await page.getByRole("button", { name: "Open Breakfast meal" }).click();
    await page.getByRole("button", { name: "Copy", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "Copy meal", exact: true });
    const submit = dialog.getByRole("button", {
      name: "Copy meal",
      exact: true,
    });
    const saving = dialog.getByRole("status");
    let requests = 0;
    let isFailing = true;
    let release: (() => void) | undefined;
    await page.route("**/api/meals/*/copy", async (route) => {
      requests += 1;
      await new Promise<void>((resolve) => {
        release = resolve;
      });
      if (isFailing) return route.fulfill({ status: 500, body: "Copy failed" });
      await route.continue();
    });
    await dialog.getByLabel("Apple amount (g)").fill("");
    await expect(submit).toBeEnabled();
    await submit.click();
    await expect(dialog).toBeVisible();
    await dialog
      .getByRole("checkbox", { name: "Apple", exact: true })
      .uncheck();
    await dialog
      .getByRole("checkbox", { name: "Apple juice Orchard" })
      .uncheck();
    await submit.click();
    expect(requests).toBe(0);
    await dialog.getByRole("checkbox", { name: "Apple", exact: true }).check();
    await dialog.getByLabel("Apple amount (g)").fill("180");
    await submit.scrollIntoViewIfNeeded();
    const submitBox = (await submit.boundingBox())!;
    try {
      await submit.click();
      await expect(saving).toHaveText("Copying...");
      await expect(saving.getByText("Copying...")).toBeInViewport();
      await expect(saving).toBeFocused();
      await expect(dialog.getByRole("button")).toHaveCount(0);
      await expect(dialog.getByRole("textbox")).toHaveCount(0);
      await page.mouse.click(
        submitBox.x + submitBox.width / 2,
        submitBox.y + submitBox.height / 2,
      );
      await page.keyboard.type("999");
      await page.keyboard.press("Escape");
      await expect(dialog).toBeVisible();
      await expect(saving).toBeVisible();
      await expect.poll(() => requests).toBe(1);
    } finally {
      release?.();
    }
    await expect(saving).toHaveCount(0);
    await expect(dialog.getByRole("alert")).toContainText(
      "Could not copy meal",
    );
    expect(await mealsFor(page, today)).toHaveLength(0);
    await expect(dialog.getByLabel("Apple amount (g)")).toHaveValue("180");
    await expect(
      dialog.getByRole("checkbox", { name: "Apple juice Orchard" }),
    ).not.toBeChecked();
    await expect(
      dialog.getByRole("button", { name: "Cancel", exact: true }),
    ).toBeEnabled();
    isFailing = false;
    try {
      await submit.click();
      await expect(saving).toHaveText("Copying...");
      if (isDesktop) await page.mouse.click(1, 1);
      await expect(dialog).toBeVisible();
      await expect.poll(() => requests).toBe(2);
    } finally {
      release?.();
    }
    await expect(dialog).not.toBeVisible();
    expect(requests).toBe(2);
    const meals = await mealsFor(page, today);
    expect(meals).toHaveLength(1);
    expect(meals[0]!.entries).toHaveLength(1);
    expect(meals[0]!.entries[0]!.amount).toBe(180);
  });
}
