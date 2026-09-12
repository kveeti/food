import type { Locator } from "@playwright/test";

import { expect, test } from "./fixtures.ts";
import { finishSetup, login, openWaterDrawer } from "./helpers.ts";

test("saves goals and shows striped amounts beyond their markers", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.getByRole("link", { name: "You" }).click();

  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/goals") &&
      response.request().method() === "PUT",
  );
  await page.getByLabel("Daily burn (kcal)").fill("40");
  await page.getByLabel("Deficit or surplus (kcal)").fill("0");
  await page.getByLabel("Water (ml)").fill("250");
  await page.getByLabel("Protein (g)").fill("120");
  expect((await saved).ok()).toBe(true);
  await expect(page.getByRole("status")).toContainText("Saved");

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

  await openWaterDrawer(page);
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

test("keeps settings controls within the mobile page", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  await page.getByRole("link", { name: "You" }).click();

  const settings = page.getByRole("main");
  const settingsScroll = page.locator("[data-settings-scroll]");
  const locale = await page.getByLabel("Locale").boundingBox();
  const date = await page.getByLabel("Start date").boundingBox();
  expect(locale).not.toBeNull();
  expect(date).not.toBeNull();
  expect(date!.width).toBeLessThanOrEqual(locale!.width);
  expect(
    await page.locator("#root").evaluate((root) => root.scrollWidth),
  ).toBeLessThanOrEqual(
    await page.locator("#root").evaluate((root) => root.clientWidth),
  );

  expect(
    await settingsScroll.evaluate((element) => element.scrollHeight),
  ).toBeGreaterThan(
    await settingsScroll.evaluate((element) => element.clientHeight),
  );
  await settingsScroll.evaluate((element) =>
    element.scrollTo(0, element.scrollHeight),
  );
  expect(
    await settingsScroll.evaluate((element) => element.scrollTop),
  ).toBeGreaterThan(0);
  expect(await page.locator("#root").evaluate((root) => root.scrollTop)).toBe(
    0,
  );

  const settingsBox = await settings.boundingBox();
  const headerBox = await settings.locator("header").first().boundingBox();
  const lastControl = await page
    .getByRole("button", { name: "Log out" })
    .boundingBox();
  const navBox = await page.getByRole("navigation").boundingBox();
  expect(settingsBox).not.toBeNull();
  expect(headerBox).not.toBeNull();
  expect(lastControl).not.toBeNull();
  expect(navBox).not.toBeNull();
  expect(headerBox!.y).toBeCloseTo(settingsBox!.y, 0);
  expect(lastControl!.y + lastControl!.height).toBeLessThan(navBox!.y);

  await settingsScroll.evaluate((element) => element.scrollTo(0, 0));
  await page.setViewportSize({ width: 900, height: 600 });
  const wideSettingsBox = await settings.boundingBox();
  expect(wideSettingsBox).not.toBeNull();
  expect(wideSettingsBox!.x).toBe(0);
  expect(wideSettingsBox!.width).toBe(900);
  await settingsScroll.evaluate((element) =>
    element.scrollTo(0, element.scrollHeight),
  );
  expect(
    await settingsScroll.evaluate((element) => element.scrollTop),
  ).toBeGreaterThan(0);
  expect(await page.evaluate(() => window.scrollY)).toBe(0);
});

test("shows save status only while saving and soon after", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  await page.getByRole("link", { name: "You" }).click();

  let releaseSave!: () => void;
  const saveGate = new Promise<void>((resolve) => {
    releaseSave = resolve;
  });
  await page.route("**/api/goals", async (route) => {
    if (route.request().method() !== "PUT") {
      await route.continue();
      return;
    }
    await saveGate;
    await route.continue();
  });

  const status = page.getByRole("status");
  await page.getByLabel("Water (ml)").fill("250");
  await expect(status).toBeEmpty();
  await expect(status).toContainText("Saving settings");

  releaseSave();
  await expect(status).toContainText("Saved");
  await expect(status).toBeEmpty({ timeout: 3_000 });
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
