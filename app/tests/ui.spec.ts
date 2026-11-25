import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?mock=1", { waitUntil: "load" });
  await page.waitForTimeout(1500);
});

test("shell landmarks and nav", async ({ page }) => {
  await expect(page.locator("header.topbar")).toBeVisible();
  await expect(page.locator("nav.rail")).toBeVisible();
  await expect(page.locator("footer.statusbar")).toBeVisible();
  await expect(page.getByRole("link", { name: "Skip to content" })).toHaveCount(1);
  await expect(page.locator('.rail-item[aria-current="page"]')).toHaveText(/Console/);
  await expect(page.getByRole("heading", { name: "Living room PS3" })).toBeVisible();
});

test("library page lists the fixture games", async ({ page }) => {
  await page.getByRole("button", { name: "Library", exact: true }).click();
  await expect(page.locator(".tile")).toHaveCount(8);
  await expect(page.locator(".tile .t").first()).toContainText(/God of War Collection/i);
});

test("library filters by system", async ({ page }) => {
  await page.getByRole("button", { name: "Library", exact: true }).click();
  await page.getByRole("tab", { name: "PS2" }).click();
  await expect(page.locator(".tile")).toHaveCount(1);
  await expect(page.locator(".tile .t").first()).toContainText(/God of War II/i);
});

test("game index wires the search source", async ({ page }) => {
  await page.getByRole("button", { name: "Game Index" }).click();
  await expect(page.getByText("God of War Collection").first()).toBeVisible();
  await expect(page.getByText("Demon's Souls").first()).toBeVisible();
});

test("downloads page lists local files", async ({ page }) => {
  await page.getByRole("button", { name: "Downloads" }).click();
  await expect(page.getByText("God of War Collection (USA).iso")).toBeVisible();
});

test("homebrew catalog renders with install buttons", async ({ page }) => {
  await page.getByRole("button", { name: "Homebrew" }).click();
  await expect(page.locator(".store-card")).toHaveCount(8);
  await expect(page.locator(".store-card .name").filter({ hasText: "webMAN MOD" })).toBeVisible();
});

test("controller exposes the pad and combos", async ({ page }) => {
  await page.getByRole("button", { name: "Controller" }).click();
  await expect(page.getByRole("heading", { name: "Virtual controller" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Cross", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "L3 + R3" })).toBeVisible();
});

test("conversions page explains the formats", async ({ page }) => {
  await page.getByRole("button", { name: "Convert" }).click();
  await expect(page.getByText("Folder to ISO")).toBeVisible();
  await expect(page.getByText("Merge parts")).toBeVisible();
});

test("updates page loads the newest package", async ({ page }) => {
  await page.getByRole("button", { name: "Updates" }).click();
  await page.getByPlaceholder("BLES01234").fill("BCUS98229");
  await page.getByRole("button", { name: "Look up" }).click();
  await expect(page.getByText("01.01")).toBeVisible();
});

test("inspector queue shows the seeded jobs", async ({ page }) => {
  await expect(page.locator(".queue-card .job-row")).toHaveCount(2);
});

test("settings exposes config controls", async ({ page }) => {
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByText("Game search backend").first()).toBeVisible();
  await expect(page.getByText("FAT32 split size (MiB)")).toBeVisible();
});

test("unslop-anything a11y pass: search inputs have accessible names", async ({ page }) => {
  await page.getByRole("button", { name: "Library", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Filter by title" })).toBeVisible();
  await page.getByRole("button", { name: "Game Index" }).click();
  await expect(page.getByRole("textbox", { name: "Search games" })).toBeVisible();
});
