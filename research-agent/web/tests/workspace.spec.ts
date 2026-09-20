import { test, expect } from "@playwright/test";

test("static UI supports the complete research and review workflow", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Set the direction" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Start research", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("group", { name: "Researcher model" }),
  ).toContainText("Mock");
  await page.screenshot({
    path: "test-results/new-research-desktop.png",
    fullPage: true,
  });
  await page
    .getByRole("textbox", { name: "Research topic" })
    .fill("Urban forests and summer heat");
  await page
    .getByRole("textbox", { name: "Focus & context" })
    .fill("Focus on peer-reviewed studies and practical urban planning.");
  await page.getByRole("group", { name: "Researcher model" }).click();
  const search = page.getByPlaceholder("Search models…");
  await search.fill("zzz-no-match");
  await expect(page.getByText("No models match your search")).toBeVisible();
  await search.fill("Mock");
  await page.getByRole("option", { name: "Mock", exact: true }).click();
  await page
    .getByRole("button", { name: "Start research", exact: true })
    .click();
  await expect(page).toHaveURL(/\?run=/);
  await expect(
    page.getByRole("heading", {
      name: "Urban forests and summer heat",
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Accept results", exact: true }),
  ).toBeVisible({ timeout: 15_000 });
  await expect(page.locator(".source-card")).toHaveCount(3);
  await page
    .getByRole("button", { name: "Request revision", exact: true })
    .click();
  await expect(page.locator(".notice-error")).toContainText(
    "Add overall or per-source feedback",
  );
  await page
    .getByRole("textbox", { name: "Feedback for source 1" })
    .fill("Include stronger evidence for cooling effects.");
  await page
    .getByRole("textbox", { name: "Overall feedback" })
    .fill("Prioritize recent studies.");
  await page.waitForResponse(
    (response) =>
      /\/api\/runs\/[^/]+$/.test(response.url()) &&
      response.request().method() === "GET",
  );
  await page.getByRole("tab", { name: "Activity", exact: true }).click();
  await page.getByRole("button", { name: "Show full log" }).click();
  await expect(
    page.getByText("Agent instructions", { exact: true }).first(),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Research team", exact: true }).click();
  await expect(page.locator(".team-card")).toHaveCount(3);
  await page.getByRole("tab", { name: /^Sources/ }).click();
  await expect(
    page.getByRole("textbox", { name: "Overall feedback" }),
  ).toHaveValue("Prioritize recent studies.");
  await expect(
    page.getByRole("textbox", { name: "Feedback for source 1" }),
  ).toHaveValue("Include stronger evidence for cooling effects.");
  await page.screenshot({
    path: "test-results/review-desktop.png",
    fullPage: true,
  });
  const feedbackRequest = page.waitForRequest(
    (request) =>
      request.url().endsWith("/feedback") && request.method() === "POST",
  );
  await page
    .getByRole("button", { name: "Request revision", exact: true })
    .click();
  expect((await feedbackRequest).postDataJSON()).toMatchObject({
    action: "revise",
    overall: "Prioritize recent studies.",
    sources: { s1: "Include stronger evidence for cooling effects." },
  });
  await expect(
    page.getByRole("button", { name: "Accept results", exact: true }),
  ).toBeVisible({ timeout: 10_000 });
  await page
    .getByRole("button", { name: "Accept results", exact: true })
    .click();
  await expect(
    page.getByText("Accepted and saved", { exact: true }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByText("Accepted and saved", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Delete research", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Delete research", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Set the direction" }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test("model discovery failures are visible and recoverable", async ({
  page,
}) => {
  await page.route("**/api/models", (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({ error: "Provider is unavailable" }),
    }),
  );
  await page.goto("/");
  await expect(page.locator(".notice-error")).toContainText(
    "Provider is unavailable",
  );
  await page.getByRole("textbox", { name: "Research topic" }).fill("A topic");
  await expect(
    page.getByRole("button", { name: "Start research", exact: true }),
  ).toBeDisabled();
  await page.unroute("**/api/models");
  await page.getByRole("button", { name: "Refresh models" }).click();
  await expect(
    page.getByRole("button", { name: "Start research", exact: true }),
  ).toBeEnabled();
});

test("mobile layout supports setup and research navigation without horizontal scrolling", async ({
  page,
  request,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Set the direction" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Open research history" }).click();
  await expect(
    page.getByRole("navigation", { name: "Research history" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Close research history" }).click();
  await expect(
    page.getByRole("navigation", { name: "Research history" }),
  ).not.toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "test-results/new-research-mobile.png",
    fullPage: true,
  });
  const created = await request.post("/api/runs", {
    data: { topic: "Mobile research review" },
  });
  const { id } = await created.json();
  await page.goto(`/?run=${id}`);
  await expect(
    page.getByRole("button", { name: "Accept results", exact: true }),
  ).toBeVisible({ timeout: 15_000 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "test-results/review-mobile.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: "Open research history" }).click();
  await page
    .getByRole("textbox", { name: "Search research" })
    .fill("Mobile research");
  await page
    .getByRole("navigation", { name: "Research history" })
    .getByRole("button", { name: /Mobile research review/ })
    .click();
  await expect(
    page.getByRole("navigation", { name: "Research history" }),
  ).not.toBeVisible();
  await request.delete(`/api/runs/${id}`);
});

test("Rust serves the static export while preserving API errors", async ({
  request,
}) => {
  const home = await request.get("/");
  expect(home.status()).toBe(200);
  expect(home.headers()["content-type"]).toContain("text/html");
  const html = await home.text();
  const assets = [
    ...html.matchAll(/(?:src|href)="([^" ]+\.(?:js|css|woff2)(?:\?[^" ]*)?)"/g),
  ].map((match) => match[1]);
  expect(assets.length).toBeGreaterThan(2);
  for (const asset of new Set(assets)) {
    const response = await request.get(asset);
    expect(response.status(), asset).toBe(200);
    expect(response.headers()["content-type"], asset).not.toContain(
      "text/html",
    );
  }
  const apiError = await request.get("/api/missing");
  expect(apiError.status()).toBe(404);
  expect(await apiError.json()).toEqual({ error: "not found" });
  expect((await request.get("/package.json")).status()).toBe(404);
  expect((await request.get("/_next/static/missing.js")).status()).toBe(404);
});
