import { expect, type Page, test } from "@playwright/test";

async function completeSettings(page: Page) {
  if (new URL(page.url()).pathname !== "/settings") return;

  await page.getByLabel("Locale").fill("en-US");
  await page.getByLabel("Timezone").fill("Europe/Helsinki");
  await page.getByRole("button", { name: "Save" }).click();
}

test("requires locale and timezone after sign in", async ({ page }) => {
  const response = await page.goto("/?meal=breakfast");
  expect(response?.headers()["x-request-id"]).toMatch(
    /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
  );

  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();

  await expect(page).toHaveURL(/\/settings\?return_to=/);
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByLabel("Locale")).not.toHaveValue("");
  await expect(page.getByLabel("Timezone")).not.toHaveValue("");
  await expect(page.getByText("Suggested from this device.")).toBeVisible();
  const invalidLocale = await page.request.post("/settings", {
    form: {
      locale: "not_a_locale",
      timezone: "Europe/Helsinki",
      return_to: "/?meal=breakfast",
    },
  });
  expect(invalidLocale.status()).toBe(400);
  const invalidTimezone = await page.request.post("/settings", {
    form: {
      locale: "en-US",
      timezone: "Not/A_Timezone",
      return_to: "/?meal=breakfast",
    },
  });
  expect(invalidTimezone.status()).toBe(400);

  await page.getByLabel("Locale").fill("en-US");
  await page.getByLabel("Timezone").fill("Europe/Helsinki");
  const saveRequest = page.waitForRequest(
    (request) =>
      request.method() === "POST" &&
      new URL(request.url()).pathname === "/settings",
  );
  await page.getByRole("button", { name: "Save" }).click();
  expect((await saveRequest).headers()["hx-request"]).toBe("true");

  await expect(page).toHaveURL("http://127.0.0.1:8200/?meal=breakfast");
  await expect(page.getByText("alice@dev.local")).toBeVisible();

  const settingsRequest = page.waitForRequest(
    (request) => new URL(request.url()).pathname === "/settings",
  );
  await page.getByRole("link", { name: "Settings" }).click();
  expect((await settingsRequest).headers()["hx-request"]).toBe("true");
  await expect(page.getByRole("button", { name: "Log out" })).toBeVisible();
  await expect(page.getByLabel("Locale")).toHaveValue("en-US");
  await expect(page.getByLabel("Timezone")).toHaveValue("Europe/Helsinki");

  const logoutRequest = page.waitForRequest(
    (request) =>
      request.method() === "POST" &&
      new URL(request.url()).pathname === "/logout",
  );
  await page.getByRole("button", { name: "Log out" }).click();
  expect((await logoutRequest).headers()["hx-request"]).toBe("true");
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

test("sign in and settings work without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({
    baseURL: "http://127.0.0.1:8200",
    javaScriptEnabled: false,
  });
  const page = await context.newPage();

  await page.goto("/");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /bob@dev\.local/ }).click();
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByLabel("Locale")).toHaveValue("");
  await expect(page.getByLabel("Timezone")).toHaveValue("");
  await expect(page.getByText("Suggested from this device.")).toBeHidden();
  await page.getByLabel("Locale").fill("en-FI");
  await page.getByLabel("Timezone").fill("Europe/Helsinki");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page).toHaveURL("http://127.0.0.1:8200/");
  await expect(page.getByText("bob@dev.local")).toBeVisible();
  await page.getByRole("link", { name: "Settings" }).click();
  await expect(page.getByLabel("Locale")).toHaveValue("en-FI");
  await expect(page.getByLabel("Timezone")).toHaveValue("Europe/Helsinki");

  await context.close();
});

test("refreshes one session safely across app processes", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /bob@dev\.local/ }).click();
  await completeSettings(page);
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
  await completeSettings(page);
  await expect(page.getByText("alice@dev.local")).toBeVisible();

  const response = await page.request.post(
    "http://127.0.0.1:8201/revoke",
    { form: { subject: "alice" } },
  );
  expect(response.ok()).toBe(true);

  await page.getByRole("link", { name: "Settings" }).click();
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Log out" })).toHaveCount(0);
});
