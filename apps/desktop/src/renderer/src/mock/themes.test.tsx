// Theme, through `App` on the mods-themes scenario: Settings chooses, the document follows, Mods
// lists what is installed, and safe mode loads none of it.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The radio and the switch draw their own mark over the input, so a pointer press lands on the mark; the input takes the press itself. */
const press = (role: "radio" | "switch", name: string) => (page.getByRole(role, { name }).element() as HTMLElement).click();
const root = () => document.documentElement;
const token = (name: string) => getComputedStyle(root()).getPropertyValue(name).trim();

afterEach(() => window.history.replaceState(null, "", window.location.pathname));

test("Light is the real token set: the root takes it, and the aliases read it", async () => {
  mount("mods-themes");
  await onScreen();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  expect(root().dataset["theme"]).toBe("dark");
  const dark = token("--surface-card");

  press("radio", "Light");
  expect(root().dataset["theme"]).toBe("light");
  expect(token("--bg-raised")).toBe("#FFFFFF");
  // An alias declared once, in semantic.css, follows the theme and not the dark value it first read.
  expect(token("--surface-card")).toBe("#FFFFFF");
  expect(token("--surface-card")).not.toBe(dark);
});

test("a mod's theme is applied from its CSS, and Dark returns when it is switched off", async () => {
  mount("mods-themes");
  await onScreen();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  press("radio", "Nord");
  expect(root().dataset["theme"]).toBe("nord");
  expect(token("--bg-base")).toBe("#2E3440");

  await page.getByRole("button", { name: "Mods", exact: true }).click();
  press("switch", "Nord");
  expect(root().dataset["theme"]).toBe("dark");
  expect(token("--bg-base")).toBe("#0F1419");
});

test("?nomods loads without mods: none in the picker, none on the Mods surface", async () => {
  window.history.replaceState(null, "", "?nomods");
  mount("mods-themes");
  await onScreen();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect.element(page.getByRole("radio", { name: "Light" })).toBeVisible();
  await expect.element(page.getByRole("radio", { name: "Nord" })).not.toBeInTheDocument();
  await page.getByRole("button", { name: "Mods", exact: true }).click();
  await expect.element(page.getByRole("group", { name: "Nord" })).not.toBeInTheDocument();
});

test("a themed element below the root resolves the aliases from its own theme, as a story's wrapper does", () => {
  const wrap = document.createElement("div");
  wrap.dataset["theme"] = "light";
  document.body.append(wrap);
  const read = (name: string) => getComputedStyle(wrap).getPropertyValue(name).trim();
  expect(read("--bg-raised")).toBe("#FFFFFF");
  expect(read("--surface-card")).toBe("#FFFFFF");
  expect(read("--text-body")).toBe("#18202A");
  expect(read("--status-running-bg")).toContain("#1B657C");
  wrap.remove();
});
