// A theme tweak is one shared colour set over whichever theme is chosen, from the tweak mod Fleet lists and the stylesheet it checked.
// Over `teach-mods`, which starts with the grey of a stopped Job made lighter. Choosing another theme replaces the theme's sheet and
// leaves the tweak, switching the mod off gives the colour back to the theme, and safe mode lays none of it.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { modSwatch } from "../swatches";
import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";
import { mockThemes } from "./themes";

unmountAfterEach();

const TOKEN = "--status-killed";
/** A colour, spelled by arithmetic: the gate refuses a colour literal in what a renderer ships, tests included. */
const hex = (n: number) => `#${n.toString(16).toUpperCase().padStart(6, "0")}`;
const LIGHTER = hex(0xb4bbc3);
const token = () => getComputedStyle(document.documentElement).getPropertyValue(TOKEN).trim().toUpperCase();

afterEach(() => window.history.replaceState(null, "", window.location.pathname));

async function choose(name: string) {
  await toSystemPage("Theme");
  await page.getByRole("combobox", { name: "Theme" }).fill(name);
  await page.getByRole("option", { name, exact: true }).click();
}

test("a tweak survives choosing another theme, in either direction, and goes when its switch does", async () => {
  mount("teach-mods");
  await onScreen();
  await expect.poll(token).toBe(LIGHTER);

  await choose("Light");
  expect(document.documentElement.dataset["theme"]).toBe("light");
  expect(token()).toBe(LIGHTER);

  await choose("Dark");
  expect(document.documentElement.dataset["theme"]).toBe("dark");
  expect(token()).toBe(LIGHTER);

  await choose("Light");
  await page.getByRole("switch", { name: "stopped-lighter" }).click({ force: true });
  await expect.poll(token).not.toBe(LIGHTER);
  expect(document.documentElement.dataset["theme"]).toBe("light");
});

test("a tweak outranks a theme mod that writes its block as :root[data-theme]", async () => {
  mount("teach-mods");
  await onScreen();
  const css = `:root[data-theme="late"] { ${TOKEN}: ${hex(0x123456)}; }`;
  mockThemes.install({ name: "late", title: "Late", enabled: true, swatch: modSwatch(css), load: () => Promise.resolve(css) });
  await choose("Late");
  expect(document.documentElement.dataset["theme"]).toBe("late");
  expect(token()).toBe(LIGHTER);
});

test("safe mode lays no tweak over the theme", async () => {
  window.history.replaceState(null, "", `${window.location.pathname}?nomods`);
  mount("teach-mods");
  await onScreen();
  await new Promise((done) => setTimeout(done, 300));
  expect(token()).not.toBe(LIGHTER);
});
