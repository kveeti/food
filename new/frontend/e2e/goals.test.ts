import type { Locator } from "@playwright/test";

import { expect, test } from "./fixtures.ts";
import { finishSetup, login } from "./helpers.ts";

test("saves goals and shows striped amounts beyond their markers", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.getByRole("link", { name: "Settings" }).click();

  await page.getByLabel("Daily burn (kcal)").fill("40");
  await page.getByLabel("Deficit or surplus (kcal)").fill("0");
  await page.getByLabel("Water (ml)").fill("250");
  await page.getByLabel("Protein (g)").fill("120");
  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/goals") &&
      response.request().method() === "PUT",
  );
  await page.getByRole("button", { name: "Save goals" }).click();
  expect((await saved).ok()).toBe(true);

  await page.getByRole("link", { name: "Today" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("0 / 40 kcal");
  await expect(page.getByLabel("Water total")).toHaveText("0 / 250 ml");
  await expect(page.getByText("0 / 120 g", { exact: true })).toBeVisible();

  const search = page.getByRole("combobox", { name: "Search foods" });
  await search.fill("apple");
  await page.getByRole("option", { name: "Apple Fineli" }).click();
  await page.getByLabel("Amount (g)").fill("100");
  await page.getByRole("combobox", { name: "Meal" }).selectOption("breakfast");
  await page.getByRole("button", { name: "Add food" }).click();
  await expect(page.getByLabel("Calorie total")).toHaveText("50 / 40 kcal");

  const glass = page.getByRole("slider", { name: "Glass amount" });
  const bounds = await glass.boundingBox();
  expect(bounds).not.toBeNull();
  await glass.click({
    position: { x: bounds!.width / 2, y: bounds!.height / 2 },
  });
  await page.getByRole("button", { name: "Add water" }).click();
  await expect(page.getByLabel("Water total")).toHaveText("300 / 250 ml");

  for (const [label, goalShare] of [
    ["Calorie total", 4 / 5],
    ["Water total", 5 / 6],
  ] as const) {
    const card = page.locator(".goal-card").filter({
      has: page.getByLabel(label),
    });
    await expect(card.locator(".goal-card-range")).toBeVisible();
    await expect
      .poll(() => rangePositions(card).then((positions) => positions.start))
      .toBeCloseTo(goalShare, 1);
    await expect
      .poll(() => rangePositions(card).then((positions) => positions.over))
      .toBeCloseTo(1 - goalShare, 1);
  }
});

async function rangePositions(card: Locator) {
  return card.evaluate((element) => {
    const cardBox = element.getBoundingClientRect();
    const overBox = element
      .querySelector(".goal-card-range")!
      .getBoundingClientRect();
    return {
      start: (overBox.left - cardBox.left) / cardBox.width,
      over: overBox.width / cardBox.width,
    };
  });
}
