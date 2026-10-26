// A pasted or dropped link goes to the page it names: a pull request to Reviews, an issue to the side panel, and anything
// else on to the link panel, which is claimed after both. A paste inside a text field is left alone. Browser: real events.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { RepositorySummary } from "@armada/protocol";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

const opened = vi.hoisted(() => ({ calls: [] as unknown[], hosted: true }));
vi.mock("./shared-panel/store", async (original) => ({
  ...(await original<typeof import("./shared-panel/store")>()),
  openInPanel: (item: unknown) => (opened.calls.push(item), opened.hosted),
}));

import { useLinkRoutes } from "./link-routes";
import { useClaim, usePasteAndDrop } from "./paste-and-drop";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const repositories = [{ root: "/work/armada", records_root: "/r", manifest: { id: "m-armada", repository: "Mele-Labs/armada" } }] as unknown as RepositorySummary[];
const roots: Root[] = [];
const pulls: unknown[] = [];
const reachedLink: string[] = [];

function Shell() {
  usePasteAndDrop();
  useLinkRoutes((link) => pulls.push(link), repositories);
  // Stands in for `link-drop.tsx`, which claims after both.
  useClaim("link", { paste: (event) => (reachedLink.push(event.clipboardData?.getData("text/plain") ?? ""), true), drop: () => (reachedLink.push("drop"), true) });
  return null;
}

beforeEach(() => {
  opened.calls.length = 0;
  opened.hosted = true;
  pulls.length = 0;
  reachedLink.length = 0;
  const root = createRoot(document.createElement("div"));
  roots.push(root);
  act(() => root.render(<Shell />));
});
afterEach(() => {
  for (const root of roots.splice(0)) act(() => root.unmount());
});

const pasteOn = (target: HTMLElement, text: string) => {
  const data = new DataTransfer();
  data.setData("text/plain", text);
  const event = new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
};
const dropOn = (target: HTMLElement, text: string) => {
  const data = new DataTransfer();
  data.setData("text/uri-list", text);
  const event = new DragEvent("drop", { dataTransfer: data, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
};

const PULL = "https://forge.example/Mele-Labs/armada/pull/2899";
const ISSUE = "https://forge.example/Mele-Labs/armada/issues/2214";

test("a pasted pull request link opens Reviews on it and goes no further", () => {
  const event = pasteOn(document.body, PULL);
  expect(pulls).toEqual([{ owner: "Mele-Labs", name: "armada", number: 2899 }]);
  expect(event.defaultPrevented).toBe(true);
  expect(reachedLink).toEqual([]);
});

test("a dropped pull request link opens Reviews on it", () => {
  dropOn(document.body, PULL);
  expect(pulls).toHaveLength(1);
  expect(reachedLink).toEqual([]);
});

test("a pasted forge issue opens the side panel on the issue, in its repository", () => {
  pasteOn(document.body, ISSUE);
  expect(opened.calls).toEqual([{ kind: "issue", number: "2214", address: ISSUE, root: "/work/armada", manifestId: "m-armada" }]);
  expect(reachedLink).toEqual([]);
});

test("a dropped tracker issue opens the side panel by its key", () => {
  dropOn(document.body, "https://tracker.example/browse/SHIP-42");
  expect(opened.calls).toEqual([{ kind: "issue", number: "SHIP-42", address: "https://tracker.example/browse/SHIP-42" }]);
});

test("an issue falls through to the link panel where no window hosts a side panel", () => {
  opened.hosted = false;
  pasteOn(document.body, ISSUE);
  expect(reachedLink).toEqual([ISSUE]);
});

test("any other link still reaches the link panel", () => {
  const event = pasteOn(document.body, "https://impeccable.style/");
  expect(reachedLink).toEqual(["https://impeccable.style/"]);
  expect(pulls).toEqual([]);
  expect(opened.calls).toEqual([]);
  expect(event.defaultPrevented).toBe(false);
  dropOn(document.body, "https://impeccable.style/");
  expect(reachedLink).toEqual(["https://impeccable.style/", "drop"]);
});

test("a pull request or issue link pasted into a text field is left to the field and the link panel's offer", () => {
  for (const make of [() => document.createElement("textarea"), () => document.createElement("input"), () => Object.assign(document.createElement("div"), { contentEditable: "true" })]) {
    const field = make();
    document.body.append(field);
    pasteOn(field, PULL);
    pasteOn(field, ISSUE);
    field.remove();
  }
  expect(pulls).toEqual([]);
  expect(opened.calls).toEqual([]);
  expect(reachedLink).toHaveLength(6);
});
