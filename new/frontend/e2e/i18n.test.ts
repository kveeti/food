import { expect, test } from "./fixtures.ts";
import { expectSameDocument, login, markDocument } from "./helpers.ts";

test("uses saved locale and timezone for dates and water", async ({ page }) => {
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

  await page
    .getByRole("link", { name: "Settings" })
    .dispatchEvent("mousedown", { button: 0 });
  await page.getByLabel("Locale").fill("en-US");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expectSameDocument(page);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "January 11, 2026",
  );
  await expect(total).toHaveText("1,500 ml");
  await expect(waterEntries).toContainText("1,500 ml");
  await expect(waterEntries.locator("time")).toHaveText(
    new Intl.DateTimeFormat("en-US", {
      hour: "numeric",
      minute: "2-digit",
      timeZone: "Pacific/Kiritimati",
    }).format(new Date(waterEntry.consumed_at)),
  );

  await page
    .getByRole("link", { name: "Settings" })
    .dispatchEvent("mousedown", { button: 0 });
  await page.getByLabel("Timezone").fill("Pacific/Honolulu");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expectSameDocument(page);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "January 10, 2026",
  );
  await page
    .getByRole("link", { name: "Next day" })
    .dispatchEvent("mousedown", { button: 0 });
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "January 11, 2026",
  );
  await expect(page.getByText("Sunday", { exact: true })).toBeVisible();
});
