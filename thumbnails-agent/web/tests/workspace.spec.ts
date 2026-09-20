import { test, expect } from "@playwright/test";

test("static UI supports the complete thumbnail workflow", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Create a thumbnail direction" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Generate concepts", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("group", { name: "Concept creator model" }),
  ).toContainText("Mock");
  await page.screenshot({
    path: "test-results/new-run-desktop.png",
    fullPage: true,
  });
  await page
    .getByRole("textbox", { name: "Video topic" })
    .fill("Why AI agents fail in production");
  await page
    .getByRole("textbox", { name: "Video summary" })
    .fill("A practical look at the failure modes and how to avoid them.");
  await page.getByRole("group", { name: "Concept creator model" }).click();
  const search = page.getByPlaceholder("Search models…");
  await search.fill("zzz-no-match");
  await expect(page.getByText("No models match your search")).toBeVisible();
  await search.fill("Mock");
  await page.getByRole("option", { name: "Mock", exact: true }).click();
  await page
    .getByRole("button", { name: "Generate concepts", exact: true })
    .click();
  await expect(page).toHaveURL(/\?run=/);
  await expect(
    page.getByRole("heading", {
      name: "Why AI agents fail in production",
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: "Accept concepts & generate images",
      exact: true,
    }),
  ).toBeVisible({ timeout: 15_000 });
  // Per-concept prompts can be copied without finishing the run.
  await expect(
    page.getByRole("button", { name: "Copy all prompts", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Copy prompt", exact: true }).first(),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Request refinement", exact: true })
    .click();
  await expect(page.locator(".notice-error")).toContainText(
    "provide feedback or choose no feedback",
  );
  await page
    .getByRole("textbox", { name: "Feedback for this concept" })
    .first()
    .fill("Make the focal subject clearer.");
  await page
    .getByRole("textbox", { name: "Overall feedback" })
    .fill("Prioritize mobile readability.");
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
  await page.getByRole("tab", { name: "Review", exact: true }).click();
  await expect(
    page.getByRole("textbox", { name: "Overall feedback" }),
  ).toHaveValue("Prioritize mobile readability.");
  await expect(
    page.getByRole("textbox", { name: "Feedback for this concept" }).first(),
  ).toHaveValue("Make the focal subject clearer.");
  await page.screenshot({
    path: "test-results/review-desktop.png",
    fullPage: true,
  });
  const feedbackRequest = page.waitForRequest(
    (request) =>
      request.url().endsWith("/feedback") && request.method() === "POST",
  );
  await page
    .getByRole("button", { name: "Request refinement", exact: true })
    .click();
  expect((await feedbackRequest).postDataJSON()).toMatchObject({
    action: "revise_concepts",
    overall: "Prioritize mobile readability.",
  });
  const stopRequest = page.waitForRequest(
    (request) =>
      request.url().endsWith("/feedback") && request.method() === "POST",
  );
  await expect(
    page.getByRole("button", {
      name: "Stop here, I’ll copy the prompts",
      exact: true,
    }),
  ).toBeVisible({ timeout: 15_000 });
  await page
    .getByRole("button", {
      name: "Stop here, I’ll copy the prompts",
      exact: true,
    })
    .click();
  expect((await stopRequest).postDataJSON()).toMatchObject({
    action: "stop_at_prompts",
  });
  await expect(
    page.getByRole("heading", { name: "Prompts ready", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Copy all prompts", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: "Accept concepts & generate images",
      exact: true,
    }),
  ).not.toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Prompts ready", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Delete run", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Create a thumbnail direction" }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test("runs can start with image generation turned off", async ({
  page,
  request,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Create a thumbnail direction" }),
  ).toBeVisible();
  const toggle = page.getByRole("switch", {
    name: "Generate thumbnail images",
  });
  await expect(toggle).toBeChecked();
  // The switch input is visually hidden; click its visible label instead.
  await page.getByText("Generate thumbnail images", { exact: true }).click();
  await expect(toggle).not.toBeChecked();
  await expect(
    page.getByRole("group", { name: "Image generator model" }),
  ).toHaveAttribute("data-disabled", "true");
  await expect(page.getByText("Image generation is off")).toBeVisible();
  await page
    .getByRole("textbox", { name: "Video topic" })
    .fill("Prompt-only run");
  await page
    .getByRole("button", { name: "Generate concepts", exact: true })
    .click();
  await expect(page).toHaveURL(/\?run=/);
  await expect(
    page.getByRole("button", {
      name: "Finish with these prompts",
      exact: true,
    }),
  ).toBeVisible({ timeout: 15_000 });
  // With generation off there is no separate stop action; accepting finishes.
  await expect(
    page.getByRole("button", {
      name: "Stop here, I’ll copy the prompts",
      exact: true,
    }),
  ).not.toBeVisible();
  await page
    .getByRole("button", { name: "Finish with these prompts", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Prompts ready", exact: true }),
  ).toBeVisible();
  const id = new URL(page.url()).searchParams.get("run")!;
  await request.delete(`/api/runs/${id}`);
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
  await page.getByRole("textbox", { name: "Video topic" }).fill("A topic");
  await expect(
    page.getByRole("button", { name: "Generate concepts", exact: true }),
  ).toBeDisabled();
  await page.unroute("**/api/models");
  await page.getByRole("button", { name: "Refresh models" }).click();
  await expect(
    page.getByRole("button", { name: "Generate concepts", exact: true }),
  ).toBeEnabled();
});

test("mobile layout supports setup and run navigation without horizontal scrolling", async ({
  page,
  request,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Create a thumbnail direction" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Open thumbnail history" }).click();
  await expect(
    page.getByRole("navigation", { name: "Thumbnail history" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Close thumbnail history" }).click();
  await expect(
    page.getByRole("navigation", { name: "Thumbnail history" }),
  ).not.toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "test-results/new-run-mobile.png",
    fullPage: true,
  });
  const created = await request.post("/api/runs", {
    data: { topic: "Mobile thumbnail review", generate_images: false },
  });
  const { id } = await created.json();
  await page.goto(`/?run=${id}`);
  await expect(
    page.getByRole("button", {
      name: "Finish with these prompts",
      exact: true,
    }),
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
  await page.getByRole("button", { name: "Open thumbnail history" }).click();
  await page
    .getByRole("textbox", { name: "Search thumbnails" })
    .fill("Mobile thumbnail");
  await page
    .getByRole("navigation", { name: "Thumbnail history" })
    .getByRole("button", { name: /Mobile thumbnail review/ })
    .click();
  await expect(
    page.getByRole("navigation", { name: "Thumbnail history" }),
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
