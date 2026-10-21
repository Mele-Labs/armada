// Report, from a right press or a menu's entry, with the annotation layer off: it comes on and
// opens the note card on the element pressed.

import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { Annotation } from "../../../shared/annotations";
import { askToReport } from "../report";
import "../styles/index.css";
import { Layer, type LayerFleet } from "./Layer";
import type { Sink } from "./sink";

const sink: Sink = {
  via: "dev server",
  list: async () => [],
  save: async () => undefined,
  remove: async () => undefined,
  root: async () => null,
  capture: async () => null,
  saveShot: async () => undefined,
  shot: async () => null,
};

/** A fleet of its own per layer, so a layer's first read of state is the one waited for. */
const fleetOf = () =>
  ({
    startSession: vi.fn(),
    sendSessionMessage: vi.fn(),
    sendSessionNote: vi.fn(),
    state: vi.fn(async () => ({ sessions: { state: "read", sessions: [] } }) as never),
    subscribe: () => () => undefined,
  }) satisfies LayerFleet;

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-noted]").forEach((one) => one.remove());
  sessionStorage.removeItem("armada.annotate.on");
});

/** The layer mounted and off, as Bridge under `pnpm dev` runs it until ⌥⌘A. */
async function layerOff(saved: Annotation[] = []) {
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const root = createRoot(host);
  const fleet = fleetOf();
  root.render(
    <StrictMode>
      <Layer sink={{ ...sink, save: async (note) => void saved.push(note) }} fleet={fleet} />
    </StrictMode>,
  );
  mounted.push({ root, host });
  await vi.waitFor(() => expect(fleet.state).toHaveBeenCalled());
}

function target(): HTMLElement {
  const one = document.createElement("button");
  one.setAttribute("data-noted", "");
  one.textContent = "Reportable";
  document.body.append(one);
  return one;
}

test("asking to report an element opens the note card on it with the layer off", async () => {
  const saved: Annotation[] = [];
  await layerOff(saved);
  const one = target();
  askToReport(one);

  await expect.element(page.getByRole("dialog", { name: "New note" })).toBeVisible();
  await userEvent.fill(page.getByRole("textbox", { name: "Note" }), "wrong colour");
  await page.getByRole("button", { name: "Save note" }).click();
  await vi.waitFor(() => expect(saved).toHaveLength(1));
  expect(saved[0]?.text).toBe("wrong colour");
  expect(saved[0]?.element.tag).toBe("button");
});

test("a right press on a plain element opens a menu with Report, and choosing it opens the note card", async () => {
  await layerOff();
  const one = target();
  one.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 40, clientY: 40 }));

  await page.getByRole("menuitem", { name: "Report" }).click();
  await expect.element(page.getByRole("dialog", { name: "New note" })).toBeVisible();
});

test("cancelling a card Report opened puts the layer away, and Escape does the same", async () => {
  await layerOff();
  const one = target();
  askToReport(one);
  await page.getByRole("button", { name: "Cancel" }).click();
  await vi.waitFor(() => expect(page.getByRole("dialog", { name: "New note" }).query()).toBeNull());
  expect(page.getByRole("status").query()).toBeNull();

  askToReport(one);
  await expect.element(page.getByRole("dialog", { name: "New note" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await vi.waitFor(() => expect(page.getByRole("dialog", { name: "New note" }).query()).toBeNull());
  expect(page.getByRole("status").query()).toBeNull();
});

test("cancelling a card the layer's own click opened leaves the layer on", async () => {
  sessionStorage.setItem("armada.annotate.on", "1");
  await layerOff();
  const one = target();
  one.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect.element(page.getByRole("status")).toBeVisible();
});

test("a right press the app handled itself opens no menu of the layer's", async () => {
  await layerOff();
  const one = target();
  one.addEventListener("contextmenu", (event) => event.preventDefault());
  one.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 40, clientY: 40 }));

  expect(page.getByRole("menuitem", { name: "Report" }).query()).toBeNull();
});
