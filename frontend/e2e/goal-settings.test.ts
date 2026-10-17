import type { Page } from "@playwright/test";

import { expect, test } from "./fixtures.ts";
import { finishSetup, login } from "./helpers.ts";

test("adds, changes, and clears a nutrient goal", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  await page.getByRole("link", { name: "You" }).click();

  await page
    .getByLabel("Add nutrient goal")
    .selectOption({ label: "Sodium (mg)" });
  await saveGoal(page, "Sodium (mg)", "2000");
  await page.reload();
  await expect(page.getByLabel("Sodium (mg)", { exact: true })).toHaveValue(
    "2000",
  );
  await page.getByRole("link", { name: "Today" }).click();
  const totals = page.getByRole("region", { name: "Day totals" });
  await expect(totals).toContainText("Sodium");
  await expect(totals.getByText("0 / 2,000 mg", { exact: true })).toBeVisible();

  await page.getByRole("link", { name: "You" }).click();
  await page
    .getByLabel("Nutrient for Sodium goal")
    .selectOption({ label: "Vitamin C (mg)" });
  await expect(page.getByLabel("Vitamin C (mg)", { exact: true })).toBeEmpty();
  await expect(page.getByLabel("Sodium (mg)", { exact: true })).toHaveCount(0);
  await saveGoal(page, "Vitamin C (mg)", "80");
  await page.reload();
  await expect(page.getByLabel("Vitamin C (mg)", { exact: true })).toHaveValue(
    "80",
  );
  await expect(page.getByLabel("Sodium (mg)", { exact: true })).toHaveCount(0);
  await page.getByRole("link", { name: "Today" }).click();
  await expect(totals).toContainText("Vitamin C");
  await expect(totals.getByText("0 / 80 mg", { exact: true })).toBeVisible();
  await expect(totals.getByText("Sodium", { exact: true })).toHaveCount(0);

  await page.getByRole("link", { name: "You" }).click();
  await saveGoal(page, "Vitamin C (mg)", "");
  await page.reload();
  await expect(page.getByLabel("Vitamin C (mg)", { exact: true })).toHaveCount(
    0,
  );
  await expect(
    page
      .getByLabel("Add nutrient goal")
      .getByRole("option", { name: "Vitamin C (mg)", exact: true }),
  ).toHaveCount(1);
  await page.getByRole("link", { name: "Today" }).click();
  await expect(totals.getByText("Vitamin C", { exact: true })).toHaveCount(0);
});

test("clears calorie, water, and default nutrient goals", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  await page.getByRole("link", { name: "You" }).click();
  await saveGoal(page, "Daily burn (kcal)", "2400");
  await saveGoal(page, "Water (ml)", "2000");
  await saveGoal(page, "Protein (g)", "120");
  await page.getByRole("link", { name: "Today" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("0 / 2,400 kcal");
  await expect(page.getByLabel("Water total")).toHaveText("0 / 2,000 ml");
  await expect(page.getByText("0 / 120 g", { exact: true })).toBeVisible();

  await page.getByRole("link", { name: "You" }).click();
  await saveGoal(page, "Daily burn (kcal)", "");
  await saveGoal(page, "Water (ml)", "");
  await saveGoal(page, "Protein (g)", "");
  await page.reload();
  await expect(page.getByLabel("Daily burn (kcal)")).toBeEmpty();
  await expect(page.getByLabel("Water (ml)")).toBeEmpty();
  await expect(page.getByLabel("Protein (g)", { exact: true })).toBeEmpty();
  await page.getByRole("link", { name: "Today" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("0 kcal");
  await expect(page.getByLabel("Water total")).toHaveText("0 ml");
  await expect(page.getByText("0 / 120 g", { exact: true })).toHaveCount(0);
});

test("applies dated goals and calorie adjustments without changing earlier days", async ({
  page,
}) => {
  await login(page);
  await page.clock.setFixedTime(new Date("2026-01-10T12:00:00Z"));
  await finishSetup(page);
  await page.getByRole("link", { name: "You" }).click();
  await page.getByLabel("Start date").fill("2026-01-08");
  await expect(page.getByLabel("Daily burn (kcal)")).toBeVisible();
  await saveGoal(page, "Daily burn (kcal)", "2400");
  await saveGoal(page, "Deficit or surplus (kcal)", "-400");
  await saveGoal(page, "Water (ml)", "1800");
  await saveGoal(page, "Protein (g)", "90");
  await expect(
    page.getByText("Food goal: 2,000 kcal", { exact: true }),
  ).toBeVisible();

  await page.getByLabel("Start date").fill("2026-01-10");
  await expect(page.getByLabel("Water (ml)")).toHaveValue("1800");
  await expect(page.getByLabel("Protein (g)", { exact: true })).toHaveValue(
    "90",
  );
  await saveGoal(page, "Deficit or surplus (kcal)", "300");
  await saveGoal(page, "Water (ml)", "2200");
  await saveGoal(page, "Protein (g)", "120");
  await expect(
    page.getByText("Food goal: 2,700 kcal", { exact: true }),
  ).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Deficit or surplus (kcal)")).toHaveValue("300");
  await page.getByRole("link", { name: "Today" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("0 / 2,700 kcal");
  await expect(page.getByLabel("Water total")).toHaveText("0 / 2,200 ml");
  await expect(page.getByText("0 / 120 g", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Previous day" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("0 / 2,000 kcal");
  await expect(page.getByLabel("Water total")).toHaveText("0 / 1,800 ml");
  await expect(page.getByText("0 / 90 g", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Previous day" }).click();
  await expect(page.getByLabel("Water total")).toHaveText("0 / 1,800 ml");
  await page.getByRole("button", { name: "Previous day" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("0 kcal");
  await expect(page.getByLabel("Water total")).toHaveText("0 ml");

  await page.getByRole("button", { name: "Next day" }).click();
  await page.getByRole("button", { name: "Next day" }).click();
  await page.getByRole("button", { name: "Next day" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("0 / 2,700 kcal");
  await expect(page.getByLabel("Water total")).toHaveText("0 / 2,200 ml");
  await page.getByRole("button", { name: "Next day" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("0 / 2,700 kcal");
  await expect(page.getByLabel("Water total")).toHaveText("0 / 2,200 ml");
});

test("keeps goal edits after a failed save and saves them after another edit", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.getByRole("link", { name: "You" }).click();
  await saveGoal(page, "Water (ml)", "1800");
  await saveGoal(page, "Protein (g)", "90");

  let isFailing = true;
  await page.route("**/api/goals", (route) =>
    route.request().method() === "PUT" && isFailing
      ? route.fulfill({ status: 500, body: "Save failed" })
      : route.continue(),
  );
  await page.getByLabel("Water (ml)").fill("2200");
  await page.getByLabel("Protein (g)", { exact: true }).fill("120");
  await expect(page.getByRole("alert")).toHaveText("Error saving goals");
  await expect(page.getByRole("status")).toContainText("Settings not saved");
  await expect(page.getByLabel("Water (ml)")).toHaveValue("2200");
  await expect(page.getByLabel("Protein (g)", { exact: true })).toHaveValue(
    "120",
  );

  isFailing = false;
  await saveGoal(page, "Water (ml)", "2300");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.reload();
  await expect(page.getByLabel("Water (ml)")).toHaveValue("2300");
  await expect(page.getByLabel("Protein (g)", { exact: true })).toHaveValue(
    "120",
  );
  await page.getByRole("link", { name: "Today" }).click();
  await expect(page.getByLabel("Water total")).toHaveText("0 / 2,300 ml");
  await expect(page.getByText("0 / 120 g", { exact: true })).toBeVisible();
});

test("keeps profile edits after a failed save and saves them after another edit", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.getByRole("link", { name: "You" }).click();

  let isFailing = true;
  await page.route("**/api/settings", (route) =>
    route.request().method() === "PUT" && isFailing
      ? route.fulfill({ status: 500, body: "Save failed" })
      : route.continue(),
  );
  await page.getByLabel("Locale", { exact: true }).fill("fi-FI");
  await page.getByLabel("Timezone", { exact: true }).fill("Pacific/Honolulu");
  await expect(page.getByRole("alert")).toHaveText("Error saving settings");
  await expect(page.getByRole("status")).toContainText("Settings not saved");
  await expect(page.getByLabel("Locale", { exact: true })).toHaveValue("fi-FI");
  await expect(page.getByLabel("Timezone", { exact: true })).toHaveValue(
    "Pacific/Honolulu",
  );

  isFailing = false;
  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/settings") &&
      response.request().method() === "PUT",
  );
  await page.getByLabel("Locale", { exact: true }).fill("en-FI");
  expect((await saved).ok()).toBe(true);
  await expect(page.getByRole("status")).toContainText("Saved");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.reload();
  await expect(page.getByLabel("Locale", { exact: true })).toHaveValue("en-FI");
  await expect(page.getByLabel("Timezone", { exact: true })).toHaveValue(
    "Pacific/Honolulu",
  );
});

async function saveGoal(page: Page, label: string, value: string) {
  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/goals") &&
      response.request().method() === "PUT",
  );
  await page.getByLabel(label).fill(value);
  expect((await saved).ok()).toBe(true);
  await expect(page.getByRole("status")).toContainText("Saved");
}
