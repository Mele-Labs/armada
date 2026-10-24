// The menu a right press opens where the app has none of its own: Mod in every build, Report only where the annotation layer is.

import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "./styles/index.css";
import type { AnnotationsDevApi } from "../../shared/annotations";
import { PointMenu } from "./point-menu";

let mounted: { root: Root; host: HTMLElement }[] = [];

beforeEach(() => {
  // A shipped build: no dev layer and not the dev server.
  vi.stubEnv("DEV", false);
  window.armadaDev = undefined;
});

afterEach(() => {
  vi.unstubAllEnvs();
  window.armadaDev = undefined;
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-pointed]").forEach((one) => one.remove());
});

/** Mounted, with its listener on: a press before the effect runs finds nobody listening. */
async function mount(): Promise<void> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  flushSync(() => root.render(<PointMenu />));
  mounted.push({ root, host });
  await new Promise((done) => setTimeout(done, 0));
}

function press(on?: (element: HTMLElement) => void): void {
  const one = document.createElement("button");
  one.setAttribute("data-pointed", "");
  one.textContent = "Pointed";
  document.body.append(one);
  on?.(one);
  one.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 40, clientY: 40 }));
}

test("a build without the dev layer offers Mod and no Report", async () => {
  await mount();
  press();
  await expect.element(page.getByRole("menuitem", { name: "Mod" })).toBeVisible();
  expect(page.getByRole("menuitem", { name: "Report" }).query()).toBeNull();
});

test("where the dev layer is, the menu offers Mod and Report", async () => {
  window.armadaDev = {} as AnnotationsDevApi;
  await mount();
  press();
  await expect.element(page.getByRole("menuitem", { name: "Mod" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "Report" })).toBeVisible();
});

test("a right press the app handled itself opens no menu", async () => {
  await mount();
  press((one) => one.addEventListener("contextmenu", (event) => event.preventDefault()));
  expect(page.getByRole("menuitem", { name: "Mod" }).query()).toBeNull();
});
