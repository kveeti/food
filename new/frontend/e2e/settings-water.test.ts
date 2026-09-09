import { expect, test } from "./fixtures.ts";
import {
  expectSameDocument,
  finishSetup,
  login,
  markDocument,
} from "./helpers.ts";

test("requires and saves locale and timezone", async ({ page }) => {
  await login(page);

  await expect(
    page.getByRole("heading", { name: "Set up Food" }),
  ).toBeVisible();
  await expect(page.getByLabel("Locale")).toHaveValue("en-US");
  await expect(page.getByLabel("Timezone")).toHaveValue("Europe/Helsinki");

  await page.getByLabel("Locale").fill("en-FI");
  await markDocument(page);
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page).toHaveURL((url) => url.pathname === "/");
  await expectSameDocument(page);

  await page
    .getByRole("link", { name: "Settings" })
    .dispatchEvent("mousedown", { button: 0 });
  await expect(page).toHaveURL(/\/settings$/);
  await expectSameDocument(page);
  await expect(page.getByLabel("Locale")).toHaveValue("en-FI");
  await expect(page.getByLabel("Timezone")).toHaveValue("Europe/Helsinki");
});

test("shows the previous water total while another day loads", async ({
  page,
}) => {
  await login(page);

  const progressPattern = /\/api\/goals\?.*include=progress/;
  let releaseInitial!: () => void;
  const initialGate = new Promise<void>((resolve) => {
    releaseInitial = resolve;
  });
  await page.route(progressPattern, async (route) => {
    await initialGate;
    await route.continue();
  });

  await finishSetup(page);
  const total = page.getByLabel("Water total");
  await expect(total).toHaveText("--");
  await expect(total).toHaveAttribute("aria-busy", "true");

  releaseInitial();
  await expect(total).toHaveText("0 ml");
  await expect(total).toHaveAttribute("aria-busy", "false");
  await page.unroute(progressPattern);

  let releasePreviousDay!: () => void;
  const previousDayGate = new Promise<void>((resolve) => {
    releasePreviousDay = resolve;
  });
  await page.route(progressPattern, async (route) => {
    await previousDayGate;
    await route.continue();
  });

  await page
    .getByRole("link", { name: "Previous day" })
    .dispatchEvent("mousedown", { button: 0 });
  await expect(total).toHaveText("0 ml");
  await expect(total).toHaveAttribute("aria-busy", "true");

  releasePreviousDay();
  await expect(total).toHaveAttribute("aria-busy", "false");
});

test("logs water with the glass and bottle controls", async ({ page }) => {
  await login(page);
  await finishSetup(page);

  const total = page.getByLabel("Water total");
  await expect(total).toHaveText("0 ml");

  const glass = page.getByRole("slider", { name: "Glass amount" });
  await expect(glass).toHaveAttribute("aria-valuenow", "250");
  const bounds = await glass.boundingBox();
  expect(bounds).not.toBeNull();
  await glass.click({
    position: { x: bounds!.width / 2, y: bounds!.height / 2 },
  });
  await expect(glass).toHaveAttribute("aria-valuenow", "300");

  const glassRequest = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/water-entries") &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Add water" }).click();
  expect((await glassRequest).ok()).toBe(true);
  await expect(total).toHaveText("300 ml");

  const entries = page.getByRole("list", { name: "Water entries" });
  await expect(entries.getByText("300 ml")).toBeVisible();

  const bottle = page.getByRole("slider", { name: "Bottle amount" });
  await bottle.click();
  await expect(bottle).toHaveAttribute("aria-current", "true");
  await expect(bottle).toHaveAttribute("aria-valuenow", "1000");
  await bottle.press("End");
  await expect(bottle).toHaveAttribute("aria-valuenow", "1500");

  const bottleRequest = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/water-entries") &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Add water" }).click();
  expect((await bottleRequest).ok()).toBe(true);
  await expect(total).toHaveText("1,800 ml");
  await expect(entries.getByText("1,500 ml")).toBeVisible();

  const deleteRequest = page.waitForResponse(
    (response) =>
      response.url().includes("/api/water-entries/") &&
      response.request().method() === "DELETE",
  );
  await page.getByRole("button", { name: /Delete 300 ml water entry/ }).click();
  expect((await deleteRequest).ok()).toBe(true);
  await expect(total).toHaveText("1,500 ml");
  await expect(entries.getByText("300 ml")).toHaveCount(0);

  await markDocument(page);
  await page
    .getByRole("link", { name: "Previous day" })
    .dispatchEvent("mousedown", { button: 0 });
  await expect(page).toHaveURL(/\?date=\d{4}-\d{2}-\d{2}$/);
  await expect(total).toHaveText("0 ml");
  await expectSameDocument(page);

  await page.getByRole("link", { name: "Today" }).dispatchEvent("touchstart");
  await expect(page).toHaveURL((url) => url.pathname === "/" && !url.search);
  await expect(total).toHaveText("1,500 ml");
  await expectSameDocument(page);
});
