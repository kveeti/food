import { expect, test } from "@playwright/test";

test("returns to the requested page after sign in", async ({ page }) => {
  const response = await page.goto("/?meal=breakfast");
  expect(response?.headers()["x-request-id"]).toMatch(
    /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
  );

  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();

  await expect(page).toHaveURL(
    "http://127.0.0.1:8200/?meal=breakfast",
  );
  await expect(page.getByText("alice@dev.local")).toBeVisible();

  await page.getByRole("button", { name: "Log out" }).click();
  await expect(page).toHaveURL("http://127.0.0.1:8200/sign-in");
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
});

test("preserves safe external request IDs", async ({ request }) => {
  const external = "edge-request-123";
  const response = await request.get("/sign-in", {
    headers: { "x-request-id": external },
  });

  expect(response.headers()["x-request-id"]).toBe(external);

  const replaced = await request.get("/sign-in", {
    headers: { "x-request-id": "x".repeat(129) },
  });
  expect(replaced.headers()["x-request-id"]).toMatch(
    /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
  );
});

test("refreshes one session safely across app processes", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /bob@dev\.local/ }).click();
  await expect(page.getByText("bob@dev.local")).toBeVisible();

  await page.waitForTimeout(2_500);
  const replica = await page.context().newPage();
  await Promise.all([
    page.reload(),
    replica.goto("http://127.0.0.1:8202/"),
  ]);

  await expect(page.getByText("bob@dev.local")).toBeVisible();
  await expect(replica.getByText("bob@dev.local")).toBeVisible();
});

test("ends a session through back-channel logout", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await expect(page.getByText("alice@dev.local")).toBeVisible();

  const response = await page.request.post(
    "http://127.0.0.1:8201/revoke",
    { form: { subject: "alice" } },
  );
  expect(response.ok()).toBe(true);

  await page.reload();
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
});
