// Theme, through `App` on the mods-themes scenario: Settings chooses, the document follows, Mods
// lists what is installed, and safe mode loads none of it.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import BUILT_IN from "@armada/tokens/themes/built-in.json";
import INDEX from "@armada/tokens/themes/index.json";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { DUSK_MOD, mockThemes } from "./themes";

unmountAfterEach();

// The values a theme says, read out of the files that say them rather than typed again here.
const AUTHORED = (
  import.meta as ImportMeta & {
    glob(patterns: string[], options: { eager: true; query: string; import: string }): Record<string, string>;
  }
).glob(
  [
    "../../../../../../packages/tokens/src/colors.css",
    "../../../../../../packages/tokens/src/light.css",
    "../../../../../../packages/tokens/themes/css/nord.css",
    "../../../../../../packages/tokens/themes/example-mod/dusk/theme.css",
  ],
  { eager: true, query: "?raw", import: "default" },
);
/** What `file` declares `name` as, first declaration. */
function authored(file: string, name: string): string {
  const css = Object.entries(AUTHORED).find(([path]) => path.endsWith(file))?.[1] ?? "";
  const found = css.match(new RegExp(`${name}:\\s*([^;]+);`));
  if (found?.[1] === undefined) throw new Error(`${file} declares no ${name}`);
  return found[1].trim();
}

const root = () => document.documentElement;
const token = (name: string) => getComputedStyle(root()).getPropertyValue(name).trim();

afterEach(() => window.history.replaceState(null, "", window.location.pathname));

/** Opens Settings and chooses a theme in the field, by typing its name and pressing its row. */
async function choose(name: string, typed = name) {
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("combobox", { name: "Theme" }).fill(typed);
  await page.getByRole("option", { name, exact: true }).click();
}

test("Light is the real token set: the root takes it, and the aliases read it", async () => {
  mount("mods-themes");
  await onScreen();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  expect(root().dataset["theme"]).toBe("dark");
  const dark = token("--surface-card");

  await choose("Light");
  expect(root().dataset["theme"]).toBe("light");
  expect(token("--bg-raised")).toBe(authored("light.css", "--bg-raised"));
  // An alias declared once, in semantic.css, follows the theme and not the dark value it first read.
  expect(token("--surface-card")).toBe(authored("light.css", "--bg-raised"));
  expect(token("--surface-card")).not.toBe(dark);
});

test("a catalogue theme is read when chosen, and is the scheme's own ground", async () => {
  mount("mods-themes");
  await onScreen();
  await choose("Nord");
  await expect.poll(() => root().dataset["theme"]).toBe("catalogue:nord");
  expect(token("--bg-base")).toBe(authored("nord.css", "--bg-base"));
});

test("the field groups themes by tone and narrows as it is typed in", async () => {
  mount("mods-themes");
  await onScreen();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("combobox", { name: "Theme" }).click();
  await expect.element(page.getByRole("group", { name: "Dark" })).toBeVisible();
  await expect.element(page.getByRole("group", { name: "Light" })).toBeVisible();
  // No mod is installed, so the group is not drawn at all.
  await expect.element(page.getByRole("group", { name: "From mods" })).not.toBeInTheDocument();
  await page.getByRole("combobox", { name: "Theme" }).fill("latte");
  await expect.element(page.getByRole("option", { name: "Catppuccin Latte" })).toBeVisible();
  await expect.element(page.getByRole("option", { name: "Catppuccin Mocha" })).not.toBeInTheDocument();
});

/** The colours of the strip on the row named `name`, as the browser paints them. */
const stripOf = (name: string) =>
  Array.from(page.getByRole("option", { name, exact: true }).element().querySelectorAll(".armada-themepick__cell")).map((cell) => getComputedStyle(cell).backgroundColor);
const rgb = (hex: string) => {
  const n = parseInt(hex.slice(1), 16);
  return `rgb(${n >> 16}, ${(n >> 8) & 255}, ${n & 255})`;
};

test("each row carries its own theme's strip, from the index and the built-in list, and the field the chosen one's", async () => {
  mount("mods-themes");
  await onScreen();
  mockThemes.install(DUSK_MOD);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("combobox", { name: "Theme" }).click();
  expect(stripOf("Nord")).toEqual(INDEX.find((one) => one.id === "catalogue:nord")!.swatch.map(rgb));
  expect(stripOf("Dark")).toEqual(BUILT_IN.dark.map(rgb));
  expect(stripOf("Light")).toEqual(BUILT_IN.light.map(rgb));
  // A mod's comes from its checked CSS: Dusk moves the ground off Dark's.
  expect(stripOf("Dusk")[0]).toBe(rgb(authored("theme.css", "--bg-base")));
  expect(stripOf("Dusk")[0]).not.toBe(rgb(BUILT_IN.dark[0]!));
});

test("a mod's theme is applied from its CSS, and Dark returns when it is switched off", async () => {
  mount("mods-themes");
  await onScreen();
  mockThemes.install(DUSK_MOD);
  await choose("Dusk");
  expect(root().dataset["theme"]).toBe("dusk");
  expect(token("--bg-base")).toBe(authored("theme.css", "--bg-base"));

  await page.getByRole("button", { name: "Mods", exact: true }).click();
  (page.getByRole("switch", { name: "Dusk" }).element() as HTMLElement).click();
  await expect.poll(() => root().dataset["theme"]).toBe("dark");
  expect(token("--bg-base")).toBe(authored("colors.css", "--bg-base"));
});

test("Promote puts the mod on a branch and the row says the branch, once", async () => {
  mount("mods-themes");
  await onScreen();
  mockThemes.install(DUSK_MOD);
  await page.getByRole("button", { name: "Mods", exact: true }).click();
  await page.getByRole("button", { name: "Promote Dusk" }).click();
  await expect.element(page.getByText(/^armada\/mod-dusk-\d+$/)).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Promote Dusk" })).toBeDisabled();
});

test("?nomods loads without mods: none in the field, none on the Mods surface, the catalogue still there", async () => {
  window.history.replaceState(null, "", "?nomods");
  mount("mods-themes");
  await onScreen();
  mockThemes.install(DUSK_MOD);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("combobox", { name: "Theme" }).click();
  await expect.element(page.getByRole("option", { name: "Nord", exact: true })).toBeVisible();
  await expect.element(page.getByRole("option", { name: "Dusk" })).not.toBeInTheDocument();
  await page.getByRole("button", { name: "Mods", exact: true }).click();
  await expect.element(page.getByRole("group", { name: "Dusk" })).not.toBeInTheDocument();
});

test("a themed element below the root resolves the aliases from its own theme, as a story's wrapper does", () => {
  const wrap = document.createElement("div");
  wrap.dataset["theme"] = "light";
  document.body.append(wrap);
  const read = (name: string) => getComputedStyle(wrap).getPropertyValue(name).trim();
  const light = (name: string) => authored("light.css", name);
  expect(read("--bg-raised")).toBe(light("--bg-raised"));
  expect(read("--surface-card")).toBe(light("--bg-raised"));
  expect(read("--text-body")).toBe(light("--fg-default"));
  expect(read("--status-running-bg")).toContain(light("--status-running"));
  wrap.remove();
});
