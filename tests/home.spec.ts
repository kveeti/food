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
  await expect(page.getByRole("navigation")).toBeVisible();
  await expect(page.getByRole("link", { name: "Today" })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Settings" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Log out" })).toBeVisible();
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

test("logs water and moves between days", async ({ page }) => {
  await page.goto("/?date=2035-01-15");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await completeSettings(page);

  await expect(page).toHaveURL("http://127.0.0.1:8200/?date=2035-01-15");
  await expect(page.getByRole("heading", { name: "January 15, 2035" }))
    .toBeVisible();
  await expect(page.getByLabel("Water total")).toHaveText("0 ml");

  const glass = page.getByRole("slider", {
    name: "Glass amount",
    exact: true,
  });
  const bounds = await glass.boundingBox();
  if (!bounds) throw new Error("water glass has no bounds");
  await page.mouse.move(
    bounds.x + bounds.width / 2,
    bounds.y + bounds.height * 0.75,
  );
  await page.mouse.down();
  await page.mouse.move(
    bounds.x + bounds.width / 2,
    bounds.y + bounds.height * 0.25,
  );
  await page.mouse.up();
  const vesselForm = glass.locator("xpath=ancestor::form");
  await expect(vesselForm.locator("output")).toHaveText("450 ml");

  const bottle = page.getByRole("slider", {
    name: "Bottle amount",
    exact: true,
  });
  await bottle.click();
  await expect(vesselForm.locator("output")).toHaveText("1,000 ml");
  await page.keyboard.press("Tab");
  await expect(glass).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(vesselForm.getByRole("button", { name: "Add water" }))
    .toBeFocused();
  await page.waitForTimeout(500);
  await glass.click();
  await expect(vesselForm.locator("output")).toHaveText("450 ml");

  await glass.press("Enter");
  await expect(page.getByLabel("Water total")).toHaveText("450 ml");
  await expect(glass).toBeFocused();

  await expect(page.getByRole("link", { name: "Previous day" }))
    .toHaveAttribute(
      "href",
      "/?date=2035-01-14",
    );
  await page.getByRole("link", { name: "Next day" }).click();
  await expect(page).toHaveURL("http://127.0.0.1:8200/?date=2035-01-16");
  await expect(page.getByLabel("Water total")).toHaveText("0 ml");
});

test("supports vessel limits, memory, keyboard input, and bottle submission", async ({ page }) => {
  await page.goto("/?date=2035-02-01");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await completeSettings(page);

  const glass = page.getByRole("slider", { name: "Glass amount" });
  const bottle = page.getByRole("slider", { name: "Bottle amount" });
  const form = glass.locator("xpath=ancestor::form");
  const output = form.locator("output");

  await expect(glass).toHaveAttribute("aria-valuemin", "10");
  await expect(glass).toHaveAttribute("aria-valuemax", "600");
  await expect(output).toHaveText("250 ml");

  await glass.press("ArrowUp");
  await expect(output).toHaveText("260 ml");
  await glass.press("Home");
  await expect(output).toHaveText("10 ml");
  await glass.press("ArrowDown");
  await expect(output).toHaveText("10 ml");
  await glass.press("End");
  await expect(output).toHaveText("600 ml");
  await glass.press("ArrowUp");
  await expect(output).toHaveText("600 ml");

  await bottle.click();
  await expect(bottle).toHaveAttribute("aria-valuemin", "100");
  await expect(bottle).toHaveAttribute("aria-valuemax", "1500");
  await expect(output).toHaveText("1,000 ml");

  await bottle.press("Home");
  await expect(output).toHaveText("100 ml");
  await bottle.press("ArrowDown");
  await expect(output).toHaveText("100 ml");
  await bottle.press("End");
  await expect(output).toHaveText("1,500 ml");
  await bottle.press("ArrowUp");
  await expect(output).toHaveText("1,500 ml");
  await bottle.press("Home");
  await bottle.press("ArrowUp");
  await expect(output).toHaveText("110 ml");

  await glass.click();
  await expect(output).toHaveText("600 ml");
  await bottle.click();
  await expect(output).toHaveText("110 ml");

  await bottle.press("Enter");
  await expect(page.getByLabel("Water total")).toHaveText("110 ml");
  await expect(bottle).toBeFocused();
});

test("honors reduced motion for the water picker", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/?date=2035-02-02");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await completeSettings(page);

  const glass = page.getByRole("slider", { name: "Glass amount" });
  await expect(glass).toHaveCSS("transition-duration", "0s");
  await expect(glass.locator(".water-wave-front")).toHaveCSS(
    "animation-name",
    "none",
  );
  await expect(glass.locator(".water-bubble").first()).toHaveCSS(
    "display",
    "none",
  );
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
  await expect(page.getByRole("slider", { name: "Glass amount" })).toBeHidden();
  await page.getByLabel("Amount (ml)").fill("370");
  await page.getByRole("button", { name: "Add water" }).click();
  await expect(page.getByLabel("Water total")).toHaveText("370 ml");
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
