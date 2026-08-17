import { expect, type Page, type TestInfo } from "@playwright/test";

export async function chooseTimezone(page: Page, timezone = "Europe/Helsinki") {
  await expect(page).toHaveURL(/\/settings$/);
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await page.getByRole("combobox", { name: "Timezone" }).selectOption(timezone);
  await page.getByRole("button", { name: "Save" }).click();
}

export async function ensureTimezone(page: Page) {
  if (new URL(page.url()).pathname === "/settings") {
    await chooseTimezone(page);
  }
}

export async function login(page: Page, testInfo: TestInfo) {
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();

  const user = `${testInfo.title}-${Date.now()}`
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-");
  await page.getByPlaceholder("new-user-sub").fill(user);
  await page.getByRole("button", { name: "Log in", exact: true }).click();

  await chooseTimezone(page);
  await expect(page).toHaveURL(/\/$/);
  await expect(
    page.getByRole("heading", { name: "Water", exact: true }),
  ).toBeVisible();
}
