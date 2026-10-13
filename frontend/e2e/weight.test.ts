import { Temporal } from "@js-temporal/polyfill";
import type { Page } from "@playwright/test";

import { expect, test } from "./fixtures.ts";
import { finishSetup, login } from "./helpers.ts";

async function openYou(page: Page) {
  await page.getByRole("link", { name: "You", exact: true }).click();
  await expect(
    page.getByRole("region", { name: "Weight", exact: true }),
  ).toBeVisible();
}

async function weighIn(page: Page, weight: string, measuredAt: string) {
  const section = page.getByRole("region", { name: "Weight", exact: true });
  await section.getByLabel("Weight (kg)").fill(weight);
  await section.getByLabel("Date and time").fill(measuredAt);
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(section.getByLabel("Weight (kg)")).toHaveValue("");
}

async function openHistory(page: Page) {
  const history = page.getByRole("list", { name: "Weight history" });
  if (!(await history.isVisible())) {
    await page
      .getByRole("button", { name: "Open weight history", exact: true })
      .click();
  }
  return history;
}

test("logs, edits, and deletes weight from You without replacing other weigh-ins", async ({
  page,
}) => {
  await login(page);
  await expect(
    page.getByRole("region", { name: "Weight", exact: true }),
  ).toHaveCount(0);
  await finishSetup(page);
  await expect(
    page.getByRole("region", { name: "Weight", exact: true }),
  ).toHaveCount(0);
  await openYou(page);
  await expect(page.getByLabel("Latest weight", { exact: true })).toContainText(
    "No weight yet",
  );

  await weighIn(page, "78.4", "2026-01-02T07:30");
  await weighIn(page, "79.1", "2026-01-01T22:00");
  await expect(page.getByLabel("Latest weight", { exact: true })).toContainText(
    "78.4 kg",
  );
  await weighIn(page, "78.6", "2026-01-02T17:00");
  await page.reload();

  const history = await openHistory(page);
  await expect(history.getByRole("listitem")).toHaveCount(3);
  await expect(history.getByRole("listitem").nth(0)).toContainText("78.6 kg");
  await expect(history.getByRole("listitem").nth(1)).toContainText("78.4 kg");
  await expect(history.getByRole("listitem").nth(2)).toContainText("79.1 kg");
  await history.getByRole("button", { name: /78\.4 kg/ }).click();

  const section = page.getByRole("region", { name: "Weight", exact: true });
  await expect(section.getByLabel("Weight (kg)")).toBeFocused();
  await expect(section.getByLabel("Date and time")).toHaveValue(
    "2026-01-02T07:30",
  );
  await section.getByLabel("Weight (kg)").fill("78.25");
  await section.getByLabel("Date and time").fill("2026-01-03T00:30");
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByLabel("Latest weight", { exact: true })).toContainText(
    "78.25 kg",
  );

  await page.reload();
  await openHistory(page);
  await history.getByRole("button", { name: /78\.25 kg/ }).click();
  await expect(section.getByLabel("Date and time")).toHaveValue(
    "2026-01-03T00:30",
  );
  await section.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByLabel("Latest weight", { exact: true })).toContainText(
    "78.6 kg",
  );
  await page.reload();
  await openHistory(page);
  await expect(history.getByRole("listitem")).toHaveCount(2);
  await expect(history.getByText("78.25 kg", { exact: true })).toHaveCount(0);
});

test("uses the saved timezone, accepts a decimal comma, and keeps input when validation fails", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await login(page);
  await page.getByLabel("Timezone").fill("America/New_York");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page).toHaveURL((url) => url.pathname === "/");
  await openYou(page);
  const section = page.getByRole("region", { name: "Weight", exact: true });
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(
    section.getByText("Enter a weight", { exact: true }),
  ).toBeVisible();
  await section.getByLabel("Weight (kg)").fill("0");
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(
    section.getByText("Weight must be greater than 0"),
  ).toBeVisible();

  await section.getByLabel("Weight (kg)").fill("78,25");
  await section.getByLabel("Date and time").fill("2026-03-08T02:30");
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(
    section.getByText(/Clock changes can skip or repeat times/),
  ).toBeVisible();
  await expect(section.getByLabel("Weight (kg)")).toHaveValue("78,25");
  await section.getByLabel("Date and time").fill("2026-01-02T23:30");
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(section.getByLabel("Weight (kg)")).toHaveValue("");
  await page.reload();
  const history = await openHistory(page);
  await expect(history.getByRole("listitem")).toHaveText(
    /78\.25 kg.*January 2, 2026.*11:30 PM/,
  );
  await history.getByRole("button", { name: /78\.25 kg/ }).click();
  await expect(section.getByLabel("Date and time")).toHaveValue(
    "2026-01-02T23:30",
  );
  await section.getByRole("button", { name: "Cancel", exact: true }).click();

  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/settings") &&
      response.request().method() === "PUT",
  );
  await page.getByLabel("Timezone").fill("Europe/Helsinki");
  await page.getByLabel("Timezone").press("Tab");
  await saved;
  await expect(history.getByRole("listitem")).toHaveText(
    /January 3, 2026.*6:30 AM/,
  );
  await history.getByRole("button", { name: /78\.25 kg/ }).click();
  await expect(section.getByLabel("Date and time")).toHaveValue(
    "2026-01-03T06:30",
  );
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await page.reload();
  await openHistory(page);
  await expect(history.getByRole("listitem")).toHaveText(
    /January 3, 2026.*6:30 AM/,
  );
});

test("sets the measurement time to now when asked", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  await openYou(page);

  const section = page.getByRole("region", { name: "Weight", exact: true });
  const measuredAt = section.getByLabel("Date and time");
  await measuredAt.fill("2020-01-02T03:04");
  await section.getByLabel("Weight (kg)").fill("78.4");

  const earliest = Date.now();
  await section.getByRole("button", { name: "Now", exact: true }).click();
  await expect(measuredAt).not.toHaveValue("2020-01-02T03:04");

  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/weight-entries") &&
      response.request().method() === "POST",
  );
  await section.getByRole("button", { name: "Save", exact: true }).click();
  const entry = (await saved).json() as Promise<{ measured_at: string }>;
  const savedTime = Date.parse((await entry).measured_at);
  expect(savedTime).toBeGreaterThanOrEqual(earliest);
  expect(savedTime).toBeLessThanOrEqual(Date.now());
});

test("charts weight history and lets the keyboard inspect readings", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await openYou(page);
  await weighIn(page, "79.2", "2026-01-01T08:00");
  await weighIn(page, "78.8", "2026-01-08T08:00");
  await weighIn(page, "78.4", "2026-01-15T08:00");

  await page.getByRole("link", { name: "Log", exact: true }).click();
  const weight = page.getByRole("region", { name: "Weight", exact: true });
  await weight.getByRole("button", { name: "All", exact: true }).click();
  await expect(weight.getByText("Trend", { exact: true })).toBeVisible();

  const chart = weight.getByRole("img", { name: /Weight chart/ });
  await chart.focus();
  const selected = page.getByRole("status", { name: "Selected weight" });
  await expect(selected).toContainText("78.4 kg");
  await chart.press("ArrowLeft");
  await expect(selected).toContainText("78.8 kg");
});

test("spaces chart date labels by day rather than by weigh-in", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await login(page);
  await finishSetup(page);
  const dates = [
    "2026-08-23",
    "2026-08-31",
    "2026-09-07",
    "2026-09-15",
    "2026-09-22",
  ];
  await page.route("**/api/weight-chart?*", (route) =>
    route.fulfill({
      json: {
        points: dates.map((day) => ({
          measured_at: `${day}T12:00:00Z`,
          weight_kg: 75,
          trend_weight_kg: 75,
        })),
        first: null,
        last: null,
        latest: null,
      },
    }),
  );
  await page.getByRole("link", { name: "Log", exact: true }).click();
  const chart = page.getByRole("img", { name: /Weight chart/ });
  await expect(chart.getByText("Aug 30")).toBeVisible();
  await expect(chart.getByText("Sep 14")).toBeVisible();
  await expect(chart.getByText("Aug 31")).toHaveCount(0);
  await expect(chart.getByText("Sep 15")).toHaveCount(0);
});

test("shows the real range change instead of comparing bucket averages", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.route("**/api/weight-chart?*", (route) =>
    route.fulfill({
      json: {
        points: [
          {
            measured_at: "2026-01-01T08:00:00Z",
            weight_kg: 75,
            trend_weight_kg: 75,
          },
          {
            measured_at: "2026-01-08T08:00:00Z",
            weight_kg: 80,
            trend_weight_kg: 78,
          },
        ],
        first: {
          id: "first",
          measured_at: "2026-01-01T08:00:00Z",
          weight_kg: 70,
        },
        last: {
          id: "last",
          measured_at: "2026-01-08T08:00:00Z",
          weight_kg: 80,
        },
        latest: {
          id: "latest",
          measured_at: "2026-01-09T08:00:00Z",
          weight_kg: 90,
        },
      },
    }),
  );
  await page.getByRole("link", { name: "Log", exact: true }).click();
  const weight = page.getByRole("region", { name: "Weight", exact: true });
  await weight.getByRole("button", { name: "All", exact: true }).click();
  await expect(weight.getByText("90 kg", { exact: true })).toBeVisible();
  await expect(weight.getByText("+10 kg", { exact: true })).toBeVisible();
});

test("uses the end of the saved local day for the chart range", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  const chartRequest = page.waitForRequest("**/api/weight-chart?*");
  await page.getByRole("link", { name: "Log", exact: true }).click();
  const request = await chartRequest;
  const to = new URL(request.url()).searchParams.get("to");
  expect(to).not.toBeNull();
  const end = Temporal.Instant.from(to!).toZonedDateTimeISO("Europe/Helsinki");
  expect(end.toPlainDate().toString()).toBe(
    Temporal.Now.zonedDateTimeISO("Europe/Helsinki").toPlainDate().toString(),
  );
  expect(end.toPlainTime().toString()).toBe("23:59:59.999999");
});

test("refreshes chart bounds at midnight in the saved timezone", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.clock.install({ time: new Date("2026-01-01T21:59:00Z") });

  const initialRequest = page.waitForRequest("**/api/weight-chart?*");
  await page.getByRole("link", { name: "Log", exact: true }).click();
  const initialTo = new URL((await initialRequest).url()).searchParams.get(
    "to",
  );
  expect(initialTo).toBe("2026-01-01T21:59:59.999999Z");

  const nextDayRequest = page.waitForRequest((request) => {
    const url = new URL(request.url());
    return (
      url.pathname === "/api/weight-chart" &&
      url.searchParams.get("to") === "2026-01-02T21:59:59.999999Z"
    );
  });
  await page.clock.fastForward(61_000);
  await nextDayRequest;
});

test("does not claim there are no weights when the chart fails to load", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await page.route("**/api/weight-chart?*", (route) =>
    route.fulfill({ status: 500, body: "Weight unavailable" }),
  );
  await page.getByRole("link", { name: "Log", exact: true }).click();
  const weight = page.getByRole("region", { name: "Weight", exact: true });
  await expect(weight.getByRole("alert")).toHaveText("Could not load weight.", {
    timeout: 15_000,
  });
  await expect(weight.getByText("No weight yet")).toHaveCount(0);
});

test("shows a retry when refreshing an empty chart fails", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  let shouldFail = false;
  await page.route("**/api/weight-chart?*", (route) => {
    if (shouldFail && new URL(route.request().url()).searchParams.has("from")) {
      return route.fulfill({ status: 500, body: "Weight unavailable" });
    }
    return route.fulfill({
      json: { points: [], first: null, last: null, latest: null },
    });
  });
  await page.getByRole("link", { name: "Log", exact: true }).click();
  const weight = page.getByRole("region", { name: "Weight", exact: true });
  await expect(weight.getByText("No weights in this 3m range.")).toBeVisible();
  shouldFail = true;
  await weight.getByRole("button", { name: "All", exact: true }).click();
  await weight.getByRole("button", { name: "3M", exact: true }).click();
  await expect(weight.getByRole("alert")).toHaveText(
    "Could not refresh weight.",
    { timeout: 15_000 },
  );
  shouldFail = false;
  await weight.getByRole("button", { name: "Try again" }).click();
  await expect(weight.getByRole("alert")).toHaveCount(0);
});

test("rolls back failed adds, edits, and deletes and lets the user retry", async ({
  page,
}) => {
  await login(page);
  await finishSetup(page);
  await openYou(page);

  let releaseRequest!: () => void;
  let shouldFail = true;
  let requestGate = new Promise<void>((resolve) => {
    releaseRequest = resolve;
  });
  await page.route(/\/api\/weight-entries(?:\/[^/]+)?$/, async (route) => {
    if (route.request().method() === "GET" || !shouldFail) {
      await route.continue();
      return;
    }
    await requestGate;
    await route.fulfill({ status: 500, body: "Weight write failed" });
  });

  const section = page.getByRole("region", { name: "Weight", exact: true });
  await section.getByLabel("Weight (kg)").fill("78.4");
  await section.getByRole("button", { name: "Save", exact: true }).click();
  const latest = page.getByLabel("Latest weight", { exact: true });
  await expect(latest).toContainText("78.4 kg");
  releaseRequest();
  await expect(section.getByRole("alert")).toHaveText(
    "Could not save weight. Try again.",
  );
  await expect(latest).toContainText("No weight yet");
  await expect(section.getByLabel("Weight (kg)")).toHaveValue("78.4");
  shouldFail = false;
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(section.getByLabel("Weight (kg)")).toHaveValue("");

  await page.reload();
  const history = await openHistory(page);
  await history.getByRole("button", { name: /78\.4 kg/ }).click();
  shouldFail = true;
  requestGate = new Promise<void>((resolve) => {
    releaseRequest = resolve;
  });
  await section.getByLabel("Weight (kg)").fill("78.6");
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(latest).toContainText("78.6 kg");
  releaseRequest();
  await expect(section.getByRole("alert")).toHaveText(
    "Could not save weight. Try again.",
  );
  await expect(latest).toContainText("78.4 kg");
  await expect(section.getByLabel("Weight (kg)")).toHaveValue("78.6");
  shouldFail = false;
  await section.getByRole("button", { name: "Save", exact: true }).click();
  await expect(section.getByLabel("Weight (kg)")).toHaveValue("");
  await page.reload();
  await openHistory(page);
  await history.getByRole("button", { name: /78\.6 kg/ }).click();

  shouldFail = true;
  requestGate = new Promise<void>((resolve) => {
    releaseRequest = resolve;
  });
  await section.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(latest).toContainText("No weight yet");
  releaseRequest();
  await expect(section.getByRole("alert")).toHaveText(
    "Could not delete weight. Try again.",
  );
  await expect(latest).toContainText("78.6 kg");
  shouldFail = false;
  await section.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(
    section.getByRole("button", { name: "Open weight history", exact: true }),
  ).toHaveAttribute("aria-expanded", "false");
  await page.reload();
  await expect(latest).toContainText("No weight yet");
});

test("can retry loading weight history", async ({ page }) => {
  await login(page);
  await finishSetup(page);
  await page.route("**/api/weight-entries?limit=10", (route) =>
    route.fulfill({ status: 500, body: "Weight unavailable" }),
  );
  await openYou(page);
  const section = page.getByRole("region", { name: "Weight", exact: true });
  await expect(section.getByRole("alert")).toHaveText(
    "Could not load weight history.",
    { timeout: 10_000 },
  );
  await page.unroute("**/api/weight-entries?limit=10");
  await section.getByRole("button", { name: "Try again", exact: true }).click();
  await expect(section.getByLabel("Latest weight")).toContainText(
    "No weight yet",
  );
  await weighIn(page, "78.4", "2026-01-02T07:30");
  await page.reload();
  await expect(page.getByLabel("Latest weight", { exact: true })).toContainText(
    "78.4 kg",
  );
});
