import { expect, test } from "./fixtures.ts";
import { finishSetup, login } from "./helpers.ts";

test("moves between Today, Log, and You", async ({ page }) => {
  await login(page);
  await finishSetup(page);

  await expect(page.getByRole("link", { name: "Today" })).toHaveAttribute(
    "aria-current",
    "page",
  );

  await page.getByRole("link", { name: "Log" }).click();
  await expect(page).toHaveURL(/\/log$/);
  await expect(page.getByRole("heading", { name: "Log" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Log" })).toHaveAttribute(
    "aria-current",
    "page",
  );

  await page.getByRole("link", { name: "You" }).click();
  await expect(page).toHaveURL(/\/settings$/);
  await expect(page.getByRole("heading", { name: "You" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Log out" })).toBeVisible();
});
