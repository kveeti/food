import { expect, test } from "./fixtures.ts";
import { finishSetup, login } from "./helpers.ts";

test("sends only amount and date and updates both water views before saving", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.goto("/?date=2026-01-10");

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
  await expect(total).toHaveText("250 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(1);
  await page
    .getByRole("link", { name: "Next day" })
    .dispatchEvent("mousedown", { button: 0 });
  await expect(total).toHaveText("0 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(0);
});

test("rolls back both water views after failed adds and deletes", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
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
  await expect(
    page.getByText("Could not save water", { exact: true }),
  ).toBeVisible();
  await expect(total).toHaveText("0 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(0);

  await page.unroute("**/api/water-entries");
  await page.getByRole("button", { name: "Add water" }).click();
  await expect(entries.getByRole("button")).toBeEnabled();
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
  } finally {
    releaseDelete();
  }
  await expect(entries.getByRole("listitem").getByRole("alert")).toHaveText(
    "error deleting entry",
  );
  await expect(total).toHaveText("250 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(1);

  await page.unroute("**/api/water-entries/*");
  await entries.getByRole("button").click();
  await expect(total).toHaveText("0 ml");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.reload();
  await expect(total).toHaveText("0 ml");
  await expect(entries.getByRole("listitem")).toHaveCount(0);
});

test("shows one list error and an unknown total when water reads fail", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  const total = page.getByLabel("Water total");
  const entries = page.getByRole("list", { name: "Water entries" });
  await expect(total).toHaveText("0 ml");
  await page.getByRole("button", { name: "Add water" }).click();
  await expect(entries.getByRole("button")).toBeEnabled();
  await expect(total).toHaveText("250 ml");

  await page
    .getByRole("link", { name: "Settings" })
    .dispatchEvent("mousedown", { button: 0 });
  await page.route("**/api/water-entries?*", (route) =>
    route.fulfill({ status: 500, body: "Water read failed" }),
  );
  await page
    .getByRole("link", { name: "Today" })
    .dispatchEvent("mousedown", { button: 0 });

  const error = page.getByRole("region", { name: "Water" }).getByRole("alert");
  await expect(error).toHaveText("Could not load the water entries.", {
    timeout: 15_000,
  });
  await expect(error).toHaveCount(1);
  await expect(total).toHaveText("--");
  // A failed refresh must not show a cached amount as a valid total.
  await expect(entries.getByText("250 ml")).toBeVisible();

  await page.reload();
  await expect(error).toHaveText("Could not load the water entries.", {
    timeout: 15_000,
  });
  await expect(total).toHaveText("--");
  await expect(entries.getByRole("listitem")).toHaveCount(0);

  await page.unroute("**/api/water-entries?*");
  await page.reload();
  await expect(total).toHaveText("250 ml");
  await expect(entries.getByText("250 ml")).toBeVisible();
  await expect(error).toHaveCount(0);
});
