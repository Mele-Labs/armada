// Deferred's thumbs: down says Fleet should not have kept the sentence, up says it was right and takes the thumbs away. A right press offers the same two. Browser: the drawing is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Issue } from "./IssuesBlock";
import { LaterBlock } from "./LaterBlock";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const later: NonNullable<Session["later"]> = [
  { id: "l1", text: "Check the retry on Windows", at: "13:00:00" },
  { id: "l2", text: "Name the fixture loader", at: "13:05:00", confirmed: true },
];
const issue: Issue = { kind: "issue", number: "31", title: "Issue 31", address: "https://forge/issues/31", state: "open" };

async function mount(issues: readonly Issue[] = []) {
  const onConfirm = vi.fn();
  const onDrop = vi.fn();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<LaterBlock later={later} issues={issues} manifestId={undefined} onRemind={() => undefined} onConfirm={onConfirm} onDrop={onDrop} />));
  roots.push({ root, host });
  act(() => host.querySelector<HTMLElement>("button.sv-later__row")!.click());
  return { host, onConfirm, onDrop };
}

const button = (host: HTMLElement, label: string) => host.querySelector<HTMLElement>(`[aria-label="${label}"]`);

test("thumbs down on an unconfirmed row drops it", async () => {
  const { host, onConfirm, onDrop } = await mount();
  act(() => button(host, "Not deferred: Check the retry on Windows")!.click());
  expect(onDrop).toHaveBeenCalledWith(later[0]);
  expect(onConfirm).not.toHaveBeenCalled();
});

test("thumbs up on an unconfirmed row confirms it", async () => {
  const { host, onConfirm, onDrop } = await mount();
  act(() => button(host, "Deferred: Check the retry on Windows")!.click());
  expect(onConfirm).toHaveBeenCalledWith(later[0]);
  expect(onDrop).not.toHaveBeenCalled();
});

test("a confirmed row has no thumbs, and an issue row has none", async () => {
  const { host } = await mount([issue]);
  expect(button(host, "Deferred: Name the fixture loader")).toBeNull();
  expect(button(host, "Not deferred: Name the fixture loader")).toBeNull();
  const row = host.querySelector('.sv-later__item[data-kind="issue"]')!;
  expect(row.querySelector('[aria-label^="Deferred"], [aria-label^="Not deferred"]')).toBeNull();
});

const rightPress = (host: HTMLElement, label: string) =>
  act(() => void host.querySelector(`li[aria-label="${label}"]`)!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 })));
const menuItem = (name: string) => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((one) => one.textContent?.trim() === name);

test("a right press offers Deferred and Not deferred, and each does what its thumb does", async () => {
  const { host, onConfirm, onDrop } = await mount();
  rightPress(host, "Check the retry on Windows");
  act(() => menuItem("Deferred")!.click());
  expect(onConfirm).toHaveBeenCalledWith(later[0]);
  rightPress(host, "Check the retry on Windows");
  act(() => menuItem("Not deferred")!.click());
  expect(onDrop).toHaveBeenCalledWith(later[0]);
});

test("a right press on a confirmed row offers Not deferred alone", async () => {
  const { host } = await mount();
  rightPress(host, "Name the fixture loader");
  expect(menuItem("Not deferred")).toBeDefined();
  expect(menuItem("Deferred")).toBeUndefined();
});
