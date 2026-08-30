import { expect, test } from "@playwright/test";

test("starts login on the configured app origin", async ({ page }) => {
  await page.goto("http://localhost:8200/");

  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await page.getByRole("link", { name: "Sign in" }).click();

  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();
  await expect(page).toHaveURL(/^http:\/\/127\.0\.0\.1:8201\/authorize/);

  await page.getByRole("link", { name: /bob@dev\.local/ }).click();

  await expect(page).toHaveURL("http://127.0.0.1:8200/");
  await expect(page.getByText("bob@dev.local")).toBeVisible();
});

test("returns after login and ends the session on logout", async ({ page }) => {
  await page.goto("/?meal=breakfast");

  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await page.getByRole("link", { name: "Sign in" }).click();

  await expect(
    page.getByRole("heading", { name: "Pick a dev user" }),
  ).toBeVisible();
  await expect(page).toHaveURL(/^http:\/\/127\.0\.0\.1:8201\/authorize/);

  await page.getByRole("link", { name: /alice@dev\.local/ }).click();

  await expect(page).toHaveURL(
    "http://127.0.0.1:8200/?meal=breakfast",
  );
  await expect(page.getByRole("heading", { name: "Food" })).toBeVisible();
  await expect(page.getByText("alice@dev.local")).toBeVisible();

  await page.waitForTimeout(2_500);
  const replica = await page.context().newPage();
  await Promise.all([
    page.reload(),
    replica.goto("http://127.0.0.1:8202/"),
  ]);
  await expect(page.getByText("alice@dev.local")).toBeVisible();
  await expect(replica.getByText("alice@dev.local")).toBeVisible();
  await replica.close();

  await page.getByRole("button", { name: "Log out" }).click();

  await expect(page).toHaveURL("http://127.0.0.1:8200/sign-in");
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Sign in" })).toBeVisible();
});

test("ends a session through OIDC back-channel logout", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await expect(page.getByText("alice@dev.local")).toBeVisible();

  const response = await page.request.post(
    "http://127.0.0.1:8201/revoke/alice",
  );
  expect(response.ok()).toBe(true);

  await page.reload();
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
});
