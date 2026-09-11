import { expect, test } from "./fixtures.ts";
import { expectSameDocument, login, markDocument } from "./helpers.ts";

test("uses saved locale and timezone for dates, food, and water", async ({
  page,
}) => {
  await login(page);
  await page.clock.setFixedTime(new Date("2026-01-10T12:30:00Z"));
  await page.getByLabel("Locale").fill("fi-FI");
  await page.getByLabel("Timezone").fill("Pacific/Kiritimati");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "11. tammikuuta 2026",
  );
  await markDocument(page);

  const water = page.getByRole("region", { name: "Water" });
  const total = page.getByLabel("Water total");
  const waterEntries = page.getByRole("list", { name: "Water entries" });
  const bottle = page.getByRole("slider", { name: "Bottle amount" });
  await bottle.click();
  await bottle.press("End");
  await expect(water.getByRole("status")).toHaveText("1\u00a0500 ml");
  await expect(bottle).toHaveAttribute(
    "aria-valuetext",
    "1\u00a0500 millilitres",
  );
  const waterSaved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/water-entries") &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Add water" }).click();
  const waterResponse = await waterSaved;
  expect(waterResponse.ok()).toBe(true);
  const waterEntry = await waterResponse.json();
  await expect(total).toHaveText("1\u00a0500 ml");
  await expect(waterEntries).toContainText("1\u00a0500 ml");
  await expect(waterEntries.locator("time")).toHaveText(
    new Intl.DateTimeFormat("fi-FI", {
      hour: "numeric",
      minute: "2-digit",
      timeZone: "Pacific/Kiritimati",
    }).format(new Date(waterEntry.consumed_at)),
  );

  const search = page.getByRole("combobox", { name: "Search foods" });
  await search.fill("apple");
  await page.getByRole("option", { name: "Apple Fineli", exact: true }).click();
  const amount = page.getByLabel("Amount (g)");
  await amount.fill("151");
  await page
    .getByRole("combobox", { name: "Meal", exact: true })
    .selectOption("breakfast");
  await expect(
    page.getByRole("region", { name: "New food", exact: true }),
  ).toContainText("75 kcal");
  const foodSaved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/food-entries") &&
      response.request().method() === "POST",
  );
  await amount.press("Enter");
  const foodResponse = await foodSaved;
  expect(foodResponse.ok()).toBe(true);
  const foodEntry = await foodResponse.json();
  const breakfast = page.getByRole("article", { name: "Breakfast" });
  const foodEntries = breakfast.getByRole("list", { name: "Breakfast foods" });
  await expect(foodEntries).toContainText("75 kcal");
  await expect(breakfast.locator("time")).toHaveText(
    new Intl.DateTimeFormat("fi-FI", {
      hour: "numeric",
      minute: "2-digit",
      timeZone: "Pacific/Kiritimati",
    }).format(new Date(foodEntry.eaten_at)),
  );

  await page
    .getByRole("link", { name: "Settings" })
    .dispatchEvent("pointerdown", { button: 0, isPrimary: true });
  await page.getByLabel("Locale").fill("en-US");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expectSameDocument(page);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "January 11, 2026",
  );
  await expect(total).toHaveText("1,500 ml");
  await expect(waterEntries).toContainText("1,500 ml");
  await expect(foodEntries).toContainText("75 kcal");
  await expect(waterEntries.locator("time")).toHaveText(
    new Intl.DateTimeFormat("en-US", {
      hour: "numeric",
      minute: "2-digit",
      timeZone: "Pacific/Kiritimati",
    }).format(new Date(waterEntry.consumed_at)),
  );

  await page
    .getByRole("link", { name: "Settings" })
    .dispatchEvent("pointerdown", { button: 0, isPrimary: true });
  await page.getByLabel("Timezone").fill("Pacific/Honolulu");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expectSameDocument(page);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "January 10, 2026",
  );
  await page
    .getByRole("link", { name: "Next day" })
    .dispatchEvent("pointerdown", { button: 0, isPrimary: true });
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "January 11, 2026",
  );
  await expect(page.getByText("Sunday", { exact: true })).toBeVisible();
});
