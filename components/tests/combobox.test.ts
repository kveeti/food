import { expect, test } from "@playwright/test";

test("loads server results and selects them with the keyboard", async ({ page }) => {
  await page.goto("/");

  const input = page.getByRole("combobox", { name: "Find a fruit" });
  await input.fill("ap");

  const popup = page.locator("[data-combobox-popup]");
  const listbox = page.getByRole("listbox");
  const status = page.getByRole("status");
  const apple = page.getByRole("option", { name: /Apple/ });
  const apricot = page.getByRole("option", { name: /Apricot/ });
  await expect(listbox).toBeVisible();
  await expect(popup).toHaveAttribute("popover", "manual");
  await expect(input).toHaveAttribute("aria-controls", "fruit-listbox");
  await expect(status).toHaveText("5 results available.");
  expect(await listbox.evaluate((element) =>
    [...element.children].every((child) => child.getAttribute("role") === "none")
  )).toBe(true);
  await expect(apple).toHaveAttribute("aria-selected", "true");

  await input.press("ArrowUp");
  await expect(listbox.getByRole("option").last()).toHaveAttribute("aria-selected", "true");
  await input.press("ArrowDown");
  await expect(apple).toHaveAttribute("aria-selected", "true");
  await input.press("ArrowDown");
  await expect(apricot).toHaveAttribute("aria-selected", "true");
  await input.dispatchEvent("keydown", { key: "Enter", isComposing: true });
  await expect(page.getByRole("heading", { name: "Apricot" })).toHaveCount(0);
  await input.press("Enter");

  await expect(page.getByRole("heading", { name: "Apricot" })).toBeVisible();
  await expect(input).toHaveValue("ap");
  await expect(listbox).toBeHidden();
  await expect(input).toBeFocused();
});

test("can turn keyboard looping off", async ({ page }) => {
  await page.goto("/");

  const combobox = page.locator("server-combobox");
  await combobox.evaluate((element) => {
    (element as HTMLElement & { loop: boolean }).loop = false;
  });
  await expect(combobox).toHaveAttribute("loop", "false");

  const input = page.getByRole("combobox", { name: "Find a fruit" });
  await input.fill("ap");
  const firstOption = page.getByRole("option").first();
  await expect(firstOption).toHaveAttribute("aria-selected", "true");
  await input.press("ArrowUp");
  await expect(firstOption).toHaveAttribute("aria-selected", "true");
});

test("keeps the loading indicator stable while typing", async ({ page }) => {
  await page.goto("/");

  const spinner = page.locator("[data-combobox-spinner]");
  await spinner.evaluate((element) => {
    const state = element as HTMLElement & { loadingStartedAt?: number; loadingDuration?: number };
    new MutationObserver(() => {
      if (element.hasAttribute("data-loading") && state.loadingStartedAt === undefined) {
        state.loadingStartedAt = performance.now();
      } else if (state.loadingStartedAt !== undefined) {
        state.loadingDuration = performance.now() - state.loadingStartedAt;
      }
    }).observe(element, { attributes: true, attributeFilter: ["data-loading"] });
  });

  const input = page.getByRole("combobox", { name: "Find a fruit" });
  await input.fill("ap");
  await expect(spinner).toHaveAttribute("data-loading", "");
  await page.waitForTimeout(200);
  await input.fill("app");
  await page.waitForTimeout(200);
  await expect(spinner).toHaveAttribute("data-loading", "");
  await expect(spinner).not.toHaveAttribute("data-loading", "", { timeout: 1_000 });
  const duration = await spinner.evaluate((element) =>
    (element as HTMLElement & { loadingDuration?: number }).loadingDuration
  );
  expect(duration).toBeGreaterThanOrEqual(490);
});

test("keeps the popover open while htmx replaces its contents", async ({ page }) => {
  await page.goto("/");

  const input = page.getByRole("combobox", { name: "Find a fruit" });
  const listbox = page.getByRole("listbox");
  await input.fill("ap");
  await expect(page.getByRole("option", { name: /Apricot/ })).toBeVisible();

  await page.route("**/fruits?*", async (route) => {
    const query = new URL(route.request().url()).searchParams.get("q");
    if (query === "app") await new Promise((resolve) => setTimeout(resolve, 300));
    await route.continue();
  });

  await input.fill("app");
  await expect(listbox).toBeVisible();
  await expect(listbox).toContainText("Apricot");
  await expect(input).toHaveAttribute("aria-busy", "true");
  await expect(listbox).toHaveAttribute("aria-busy", "true");
  await expect(listbox.getByRole("option").first()).toHaveAttribute("aria-selected", "true");
  await expect(listbox).not.toContainText("Apricot");
  await expect(input).not.toHaveAttribute("aria-busy", "true");
  await expect(listbox).not.toHaveAttribute("aria-busy", "true");
  await expect(listbox.getByRole("option").first()).toHaveAttribute("aria-selected", "true");
  await expect(listbox).toBeVisible();
});

test("escape closes results and typing opens fresh results", async ({ page }) => {
  await page.goto("/");

  const input = page.getByRole("combobox", { name: "Find a fruit" });
  const popup = page.locator("[data-combobox-popup]");
  const listbox = page.getByRole("listbox");
  await input.fill("berry");
  await expect(page.getByRole("option", { name: /Blackberry/ })).toBeVisible();

  await input.press("Escape");
  await expect(listbox).toBeHidden();
  await expect(input).toHaveAttribute("aria-expanded", "false");

  await input.fill("zz");
  await expect(page.getByRole("status")).toHaveText("No fruit matched “zz”.");
  await expect(page.getByRole("status")).toBeVisible();
  await expect(listbox.getByRole("option")).toHaveCount(0);
  await expect(popup).toBeVisible();
});

test("search and selection work without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto("http://127.0.0.1:3001/");

  const input = page.getByRole("searchbox", { name: "Find a fruit" });
  await input.fill("ap");
  await page.getByRole("button", { name: "Search" }).click();

  await expect(page.getByRole("link", { name: /Apple/ })).toBeVisible();
  await page.getByRole("link", { name: /Apricot/ }).click();
  await expect(page.getByRole("heading", { name: "Apricot" })).toBeVisible();
  await expect(page).toHaveURL(/fruit=apricot/);

  await context.close();
});
