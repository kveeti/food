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

  await page.locator("[data-water-stage]").scrollIntoViewIfNeeded();
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

test("searches foods and logs meals with the keyboard", async ({ page }) => {
  await page.goto("/?date=2035-03-01");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /alice@dev\.local/ }).click();
  await completeSettings(page);

  const input = page.getByRole("combobox", { name: "Search foods" });
  const listbox = page.getByRole("listbox");
  const status = listbox.locator("xpath=following-sibling::*[@role='status']");
  await input.fill("a");
  await expect(status).toBeVisible();
  await expect(status).toHaveText("Type at least two characters.");
  await expect(listbox.getByRole("option")).toHaveCount(0);

  await input.fill("milk");
  const option = page.getByRole("option", { name: /Maito, rasvaton/ });
  await expect(listbox).toBeVisible();
  await expect(input).toHaveAttribute("aria-expanded", "true");
  await expect(option).toHaveAttribute("aria-selected", "true");
  await expect(status).toHaveText("2 results available.");

  await input.press("ArrowUp");
  await expect(listbox.getByRole("option").last()).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await input.press("ArrowDown");
  await expect(option).toHaveAttribute("aria-selected", "true");
  await input.press("Enter");
  await expect(
    page.getByRole("heading", { name: "Maito, rasvaton" }),
  ).toBeVisible();
  await expect(
    page.locator("#food-preview").getByText("35 kcal", { exact: true }).first(),
  ).toBeVisible();
  await expect(input).toHaveValue("milk");
  await expect(listbox).toBeHidden();
  await expect(page).toHaveURL(/date=2035-03-01.*q=milk.*food=/);

  await page.reload();
  await expect(input).toHaveAttribute("aria-expanded", "false");
  await expect(listbox).toBeHidden();

  const diary = page.locator("#food-diary");
  const amount = page.getByRole("spinbutton", { name: "Amount", exact: true });
  const meal = page.getByLabel("Meal");
  await expect(amount).toBeFocused();
  await expect(page.getByText("All nutrients")).toBeVisible();
  await page.getByText("All nutrients").click();
  await expect(page.getByText("40 mg", { exact: true })).toBeVisible();

  await meal.selectOption("breakfast");
  await amount.fill("250");
  await expect(
    page.locator("#food-preview [data-food-kcal-output]"),
  ).toHaveText("87 kcal");
  await page.getByRole("button", { name: "Add food" }).click();
  await expect(diary.getByLabel("Food energy total")).toHaveText("87 kcal");
  await expect(diary.getByText("250 g", { exact: true })).toBeVisible();
  await expect(diary.getByRole("heading", { name: "Breakfast" })).toBeVisible();
  await expect(diary.locator("[data-meal]")).toHaveCount(1);
  await expect(amount).toBeHidden();
  await expect(input).toHaveValue("");
  await expect(input).toBeFocused();
  await expect(page).toHaveURL("http://127.0.0.1:8200/?date=2035-03-01");

  const firstEntry = diary.locator("[data-food-entry-details]").first();
  await firstEntry.locator("summary").click();
  await expect(firstEntry).toHaveAttribute("open", "");
  const editAmount = page.getByRole("spinbutton", {
    name: "Amount",
    exact: true,
  });
  await expect(editAmount).toBeFocused();
  await expect(diary.getByRole("button", { name: "Save" })).toBeVisible();
  await expect(diary.getByRole("button", { name: "Delete" })).toBeVisible();
  await expect(diary.getByRole("link", { name: "Cancel" })).toBeVisible();
  await editAmount.fill("200");
  await expect(firstEntry.locator("[data-food-kcal-output]")).toHaveText(
    "70 kcal",
  );
  const heldUpdate = Promise.withResolvers<void>();
  const updateRoute = /\/food-entries\/[^/]+$/;
  await page.route(updateRoute, async (route) => {
    await heldUpdate.promise;
    await route.continue();
  });
  await editAmount.press("Enter");
  await expect(firstEntry).toBeVisible();
  await expect(diary.getByLabel("Food energy total")).toHaveCSS(
    "filter",
    "blur(1px)",
  );
  heldUpdate.resolve();
  await expect(diary.getByLabel("Food energy total")).toHaveText("70 kcal");
  await page.unroute(updateRoute);
  await expect(diary.getByText("200 g", { exact: true })).toBeVisible();

  await diary.locator("[data-food-entry-details] summary").first().click();
  await editAmount.fill("300");
  await diary.getByRole("link", { name: "Cancel" }).click();
  await expect(diary.getByText("200 g", { exact: true })).toBeVisible();

  await firstEntry.locator("summary").click();
  await expect(firstEntry).toHaveAttribute("open", "");
  await firstEntry.locator("summary").click();
  await expect(firstEntry).not.toHaveAttribute("open", "");

  await input.fill("milk");
  await expect(page.getByRole("option", { name: /Maito, rasvaton/ }))
    .toBeVisible();
  await input.press("Enter");
  await expect(meal).toHaveValue(/^continue:/);
  await expect(meal.locator('option[value^="continue:"]')).toHaveText(
    "Continue Breakfast",
  );
  await amount.fill("100");
  await page.getByRole("button", { name: "Add food" }).click();
  await expect(diary.getByLabel("Food energy total")).toHaveText("105 kcal");
  await expect(diary.locator("[data-meal]")).toHaveCount(1);
  await expect(diary.getByText("Maito, rasvaton", { exact: true })).toHaveCount(
    2,
  );
  await expect(diary.locator("[data-meal]").first().locator("li")).toHaveText([
    /100 g/,
    /200 g/,
  ]);
  await expect(amount).toBeHidden();
  await expect(input).toHaveValue("");
  await expect(input).toBeFocused();

  await input.fill("milk");
  await expect(page.getByRole("option", { name: /Maito, rasvaton/ }))
    .toBeVisible();
  await input.press("Enter");
  await meal.selectOption("dinner");
  await page.getByRole("button", { name: "Add food" }).click();
  await expect(diary.getByLabel("Food energy total")).toHaveText("140 kcal");
  await expect(diary.locator("[data-meal]")).toHaveCount(2);
  await expect(diary.locator("[data-meal] h3")).toHaveText([
    "Dinner",
    "Breakfast",
  ]);
  await expect(amount).toBeHidden();
  await expect(input).toHaveValue("");
  await expect(input).toBeFocused();

  const dinner = diary.locator("[data-meal]").first();
  const dinnerFood = dinner.locator(":scope > ul > li");
  const foodTotal = diary.getByLabel("Food energy total");
  await dinner.locator("[data-food-entry-details] summary").click();
  const heldDelete = Promise.withResolvers<void>();
  const deleteRoute = /\/food-entries\/[^/]+\/delete$/;
  let failDelete = true;
  await page.route(deleteRoute, async (route) => {
    if (failDelete) {
      failDelete = false;
      await route.abort("connectionfailed");
      return;
    }
    await heldDelete.promise;
    await route.continue();
  });
  const failedDelete = page.waitForEvent(
    "requestfailed",
    (request) => deleteRoute.test(request.url()),
  );
  await diary.getByRole("button", { name: "Delete" }).click();
  await failedDelete;
  await expect(dinnerFood).toBeVisible();
  await expect(foodTotal).toHaveCSS("filter", "none");

  await diary.getByRole("button", { name: "Delete" }).click();
  await expect(dinnerFood).toBeHidden();
  await expect(dinner).toBeHidden();
  await expect(foodTotal).toHaveCSS("filter", "blur(1px)");
  heldDelete.resolve();
  await expect(foodTotal).toHaveText("105 kcal");
  await expect(foodTotal).toHaveCSS("filter", "none");
  await page.unroute(deleteRoute);
  await expect(diary.locator("[data-meal] h3")).toHaveText(["Breakfast"]);

  await input.fill("milk");
  await expect(page.getByRole("option", { name: /Maito, rasvaton/ }))
    .toBeVisible();
  await input.press("Enter");
  await page.getByRole("link", { name: "Cancel" }).click();
  await expect(amount).toBeHidden();
  await expect(input).toBeFocused();

  await page.reload();
  await expect(input).toHaveAttribute("aria-expanded", "false");
  await expect(listbox).toBeHidden();

  await page.goto("/?date=2035-03-02");
  const nextInput = page.getByRole("combobox", { name: "Search foods" });
  await nextInput.fill("milk");
  await expect(page.getByRole("option", { name: /Maito, rasvaton/ }))
    .toBeVisible();
  await nextInput.press("Enter");
  const nextMeal = page.getByLabel("Meal");
  await expect(nextMeal).toHaveValue("breakfast");
  await expect(nextMeal.locator('option[value^="continue:"]')).toHaveText(
    "Continue Breakfast",
  );
  await nextMeal.selectOption({ label: "Continue Breakfast" });
  await page.getByRole("button", { name: "Add food" }).click();
  await expect(diary.locator("[data-meal]")).toHaveCount(1);
  await expect(diary.getByRole("heading", { name: "Breakfast" })).toBeVisible();

  await nextInput.fill("milk");
  await expect(page.getByRole("option", { name: /Maito, rasvaton/ }))
    .toBeVisible();
  await nextInput.press("Enter");
  await expect(nextMeal).toHaveValue(/^continue:/);
  const staleMeal = await nextMeal.inputValue();

  const otherPage = await page.context().newPage();
  await otherPage.goto("/?date=2035-03-02");
  const otherInput = otherPage.getByRole("combobox", { name: "Search foods" });
  await otherInput.fill("milk");
  await expect(otherPage.getByRole("option", { name: /Maito, rasvaton/ }))
    .toBeVisible();
  await otherInput.press("Enter");
  await otherPage.getByLabel("Meal").selectOption("dinner");
  await otherPage.getByRole("button", { name: "Add food" }).click();
  await expect(otherPage.locator("[data-meal] h3").first()).toHaveText(
    "Dinner",
  );
  await otherPage.close();

  await expect(nextMeal).toHaveValue(staleMeal);
  await page.getByRole("button", { name: "Add food" }).click();
  await expect(diary.locator("[data-meal] h3")).toHaveText([
    "Breakfast",
    "Dinner",
    "Breakfast",
  ]);
});

test("food search and selection work without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({
    baseURL: "http://127.0.0.1:8200",
    javaScriptEnabled: false,
  });
  const page = await context.newPage();

  await page.goto("/?date=2035-03-02");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.getByRole("link", { name: /bob@dev\.local/ }).click();
  await completeSettings(page);

  const input = page.getByRole("searchbox", { name: "Search foods" });
  await input.fill("apple");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await page.getByRole("button", { name: /Omena, keskiarvo/ }).click();

  await expect(
    page.getByRole("heading", { name: "Omena, keskiarvo" }),
  ).toBeVisible();
  await expect(input).toHaveValue("apple");
  await expect(page).toHaveURL(/date=2035-03-02.*q=apple.*food=/);

  const amount = page.getByRole("spinbutton", { name: "Amount", exact: true });
  await expect(amount).toBeFocused();
  await page.getByLabel("Meal").selectOption("lunch");
  await amount.fill("200");
  await page.getByRole("button", { name: "Add food" }).click();
  await expect(page).toHaveURL("http://127.0.0.1:8200/?date=2035-03-02");
  await expect(page.getByLabel("Food energy total")).toHaveText("88 kcal");
  await expect(page.getByText("200 g", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Lunch" })).toBeVisible();

  const diary = page.locator("#food-diary");
  await diary.locator("[data-food-entry-details] summary").click();
  const editAmount = page.getByRole("spinbutton", {
    name: "Amount",
    exact: true,
  });
  await expect(diary.locator("[data-food-kcal-output]")).toHaveText("-- kcal");
  await editAmount.fill("150");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByLabel("Food energy total")).toHaveText("66 kcal");
  await expect(page.getByText("150 g", { exact: true })).toBeVisible();

  await diary.locator("[data-food-entry-details] summary").click();
  await page.getByRole("button", { name: "Delete" }).click();
  await expect(page.getByText("No food logged.")).toBeVisible();
  await expect(diary.locator("[data-meal]")).toHaveCount(0);

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
