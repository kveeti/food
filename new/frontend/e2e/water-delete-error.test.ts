import { expect, test } from "./fixtures.ts";
import { finishSetup, login } from "./helpers.ts";

test("deletion errors stay on each failed row", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  const entries = page.getByRole("list", { name: "Water entries" });
  const total = page.getByLabel("Water total");
  await page.getByRole("button", { name: "Add water" }).click();
  await expect(entries.getByRole("button")).toBeEnabled();
  await page.getByRole("slider", { name: "Bottle amount" }).click();
  await page.getByRole("button", { name: "Add water" }).click();
  const glass = entries.getByRole("listitem").filter({ hasText: "250 ml" });
  const bottle = entries.getByRole("listitem").filter({ hasText: "1,000 ml" });
  await expect(bottle.getByRole("button")).toBeEnabled();
  await expect(total).toHaveText("1,250 ml");

  await page.route("**/api/water-entries/*", (route) =>
    route.fulfill({ status: 500, body: "Delete failed" }),
  );
  await glass.getByRole("button").click();
  await expect(glass.getByRole("alert")).toHaveText("Error deleting water");
  await expect(bottle.getByRole("alert")).toHaveCount(0);
  await expect(bottle.getByRole("button")).toBeEnabled();
  await expect(total).toHaveText("1,250 ml");

  await bottle.getByRole("button").click();
  await expect(bottle.getByRole("alert")).toHaveText("Error deleting water");
  await expect(glass.getByRole("alert")).toHaveText("Error deleting water");
  await expect(total).toHaveText("1,250 ml");
  await page.reload();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(entries.getByRole("listitem")).toHaveCount(2);
  await expect(total).toHaveText("1,250 ml");
});
