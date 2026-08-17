import { expect, test } from "@playwright/test";

import { login } from "./helpers";

test("food search previews scaled nutrients without saving", async ({ page }, testInfo) => {
  await login(page, testInfo);

  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  const result = page.getByRole("link", { name: /SOKERI/ });
  await expect(result).toBeVisible();
  await expect(result).toContainText("406 kcal");
  await result.click();

  await expect(page.getByRole("heading", { name: "SOKERI" })).toBeVisible();
  await page.getByRole("spinbutton", { name: "Amount in grams" }).fill("50");
  await expect(page.getByText("203 kcal", { exact: true })).toBeVisible();
  await expect(page.getByText("Preview only — nothing will be saved.")).toBeVisible();

  await page.getByRole("searchbox", { name: "Search foods" }).fill("nekta");
  await expect(
    page.getByRole("link", { name: /Persikka\/nektariini/ }),
  ).toBeVisible();

  await page.getByRole("searchbox", { name: "Search foods" }).fill("maito rasvaton");
  await expect(page.locator("#food-results a").first()).toContainText(
    "Maito, rasvaton",
  );
});

test("food search and preview work without JavaScript", async ({ browser }, testInfo) => {
  const context = await browser.newContext({
    baseURL: "http://127.0.0.1:8200",
    javaScriptEnabled: false,
  });
  const page = await context.newPage();

  await login(page, testInfo);
  await page.getByRole("searchbox", { name: "Search foods" }).fill("sugar");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await page.getByRole("link", { name: /SOKERI/ }).click();
  await page.getByRole("spinbutton", { name: "Amount in grams" }).fill("50");
  await page.getByRole("button", { name: "Preview", exact: true }).click();

  await expect(page.getByText("203 kcal", { exact: true })).toBeVisible();
  await expect(page.getByText("Preview only — nothing will be saved.")).toBeVisible();
  await context.close();
});
