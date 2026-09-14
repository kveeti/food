import { expect, test } from "./fixtures.ts";
import {
  closeWaterDrawer,
  finishSetup,
  login,
  openWaterDrawer,
} from "./helpers.ts";

test("sends only amount and date and updates both water views before saving", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.goto("/?date=2026-01-10");
  await page.getByRole("button", { name: "Log water" }).click();
  await expect(page.getByRole("dialog", { name: "Water" })).toBeVisible();

  const total = page.getByLabel("Water total");
  const entries = page.getByRole("list", { name: "Water entries" });
  await expect(total).toHaveText("0 ml");

  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/water-entries", async (route) => {
    expect(route.request().postDataJSON()).toEqual({
      amount_ml: 250,
      date: "2026-01-10",
    });
    await gate;
    await route.continue();
  });
  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/water-entries") &&
      response.request().method() === "POST",
  );
  try {
    await page.getByRole("button", { name: "Add water" }).click();
    await expect(total).toHaveText("250 ml");
    await expect(entries.getByRole("listitem")).toHaveCount(1);
    await expect(entries.getByText("250 ml")).toBeVisible();
    await expect(entries.getByRole("button")).toBeDisabled();
  } finally {
    release();
  }
  expect((await saved).ok()).toBe(true);
  await expect(entries.getByRole("button")).toBeEnabled();
  await expect(entries.getByRole("listitem")).toHaveCount(1);
  await expect(total).toHaveText("250 ml");

  await page.reload();
  await openWaterDrawer(page);
  await expect(total).toHaveText("250 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(1);
  await closeWaterDrawer(page);
  await page.getByRole("button", { name: "Next day" }).click();
  await expect(total).toHaveText("0 ml");
  await openWaterDrawer(page);
  await expect(entries.getByRole("listitem")).toHaveCount(0);
});

test("rolls back both water views after failed adds and deletes", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await openWaterDrawer(page);
  const total = page.getByLabel("Water total");
  const entries = page.getByRole("list", { name: "Water entries" });
  await expect(total).toHaveText("0 ml");

  let releaseAdd!: () => void;
  const addGate = new Promise<void>((resolve) => {
    releaseAdd = resolve;
  });
  await page.route("**/api/water-entries", async (route) => {
    await addGate;
    await route.fulfill({ status: 500, body: "Could not save water" });
  });
  try {
    await page.getByRole("button", { name: "Add water" }).click();
    await expect(total).toHaveText("250 ml");
    await expect(entries.getByRole("listitem")).toHaveCount(1);
  } finally {
    releaseAdd();
  }
  const addError = page
    .getByRole("dialog", { name: "Water" })
    .getByRole("alert");
  await expect(addError).toHaveText("Error adding water");
  await expect(total).toHaveText("0 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(0);

  await page.unroute("**/api/water-entries");
  await page.getByRole("button", { name: "Add water" }).click();
  await expect(entries.getByRole("button")).toBeEnabled();
  await expect(addError).toHaveCount(0);
  await expect(total).toHaveText("250 ml");

  let releaseDelete!: () => void;
  const deleteGate = new Promise<void>((resolve) => {
    releaseDelete = resolve;
  });
  await page.route("**/api/water-entries/*", async (route) => {
    await deleteGate;
    await route.fulfill({ status: 500, body: "Could not delete water" });
  });
  try {
    await entries.getByRole("button").click();
    await expect(total).toHaveText("0 ml");
    await expect(entries.getByRole("listitem")).toHaveCount(0);
    await expect(page.getByText("No water logged for this day")).toBeVisible();
  } finally {
    releaseDelete();
  }
  await expect(entries.getByRole("listitem").getByRole("alert")).toHaveText(
    "Error deleting water",
  );
  await expect(total).toHaveText("250 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(1);
  await expect(page.getByText("No water logged for this day")).toHaveCount(0);

  await page.unroute("**/api/water-entries/*");
  let releaseSuccessfulDelete!: () => void;
  const successfulDeleteGate = new Promise<void>((resolve) => {
    releaseSuccessfulDelete = resolve;
  });
  await page.route("**/api/water-entries/*", async (route) => {
    await successfulDeleteGate;
    await route.continue();
  });
  const deleted = page.waitForResponse(
    (response) => response.request().method() === "DELETE",
  );
  await entries.getByRole("button").click();
  await expect(total).toHaveText("0 ml");
  const empty = page.getByText("No water logged for this day");
  await expect(empty).toBeVisible();
  await expect(entries.getByRole("listitem")).toHaveCount(0);
  releaseSuccessfulDelete();
  expect((await deleted).ok()).toBe(true);
  await expect(entries.getByRole("listitem")).toHaveCount(0);
  await page.reload();
  await openWaterDrawer(page);
  await expect(total).toHaveText("0 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(0);
});

test("keeps the server total when the water entry list fails", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await openWaterDrawer(page);
  const total = page.getByLabel("Water total");
  const entries = page.getByRole("list", { name: "Water entries" });
  await expect(total).toHaveText("0 ml");
  await page.getByRole("button", { name: "Add water" }).click();
  await expect(entries.getByRole("button")).toBeEnabled();
  await expect(total).toHaveText("250 ml");
  await closeWaterDrawer(page);

  await page
    .getByRole("link", { name: "You" })
    .dispatchEvent("pointerdown", { button: 0, isPrimary: true });
  let failing = true;
  await page.route("**/api/water-entries?*", (route) => {
    if (failing) {
      return route.fulfill({ status: 500, body: "Water read failed" });
    }
    return route.continue();
  });
  await page
    .getByRole("link", { name: "Today" })
    .dispatchEvent("pointerdown", { button: 0, isPrimary: true });

  await openWaterDrawer(page);
  const water = page.getByRole("dialog", { name: "Water" });
  const error = water.getByRole("alert");
  await expect(error).toContainText("Error loading water entries", {
    timeout: 15_000,
  });
  await expect(error).toHaveCount(1);
  await expect(total).toHaveText("250 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(0);

  failing = false;
  await water.getByRole("button", { name: "Try again" }).click();
  await expect(total).toHaveText("250 ml");
  await expect(entries.getByText("250 ml")).toBeVisible();
  await expect(error).toHaveCount(0);
});
