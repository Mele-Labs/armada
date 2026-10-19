// A thread not read yet draws the shape of one and nothing a person could act on; one read before and read again draws its
// rows with a mark where the newer ones land. The thread is told which by `transcript`, and says nothing without it.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { SessionThread } from "@armada/components";
import type { SessionThreadRow } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

const rows: SessionThreadRow[] = [
  { id: "r1", at: "10:00:00", kind: "message", from: "you", text: "Fix the flaky clock test" },
  { id: "r2", at: "10:05:00", kind: "message", from: "agent", text: "Store::open takes a clock now" },
];

const thread = (transcript?: "unread" | "refreshing", shown: SessionThreadRow[] = rows) => (
  <SessionThread sessionId="a" rows={shown} {...(transcript === undefined ? {} : { transcript })} onAnswer={() => {}} onOpenSession={() => {}} />
);

const list = () => document.querySelector<HTMLElement>(".armada-helm-thread__rows")!;
const mark = () => page.getByRole("img", { name: "Loading new messages" });

test("an unread thread draws no rows, and its list is busy", async () => {
  mount(thread("unread", []));

  await expect.poll(() => document.querySelector(".armada-session-thread__skeleton") !== null).toBe(true);
  expect(list().getAttribute("aria-busy")).toBe("true");
  // The skeleton is the whole list: no row, no Show earlier, no mark.
  expect([...list().children].map((one) => one.className)).toEqual(["armada-session-thread__skeleton"]);
  expect(mark().query()).toBeNull();
});

test("the skeleton of an unread thread is hidden from a screen reader and holds nothing to focus", async () => {
  mount(thread("unread", []));

  await expect.poll(() => document.querySelector(".armada-session-thread__skeleton") !== null).toBe(true);
  const skeleton = document.querySelector<HTMLElement>(".armada-session-thread__skeleton")!;
  expect(skeleton.getAttribute("aria-hidden")).toBe("true");
  expect(skeleton.querySelectorAll('a, button, input, textarea, select, [tabindex], [contenteditable="true"]')).toHaveLength(0);
  // The "Loading" status its bars carry is not announced, as the list's own busy state says it once.
  expect(page.getByRole("status", { name: "Loading" }).query()).toBeNull();
});

test("a thread with rows and no transcript draws them, not busy, with no skeleton and no mark", async () => {
  mount(thread());

  await expect.element(page.getByText("Store::open takes a clock now")).toBeVisible();
  expect(list().hasAttribute("aria-busy")).toBe(false);
  expect(document.querySelector(".armada-session-thread__skeleton")).toBeNull();
  expect(mark().query()).toBeNull();
});

test("a thread read again draws its rows, then a mark named Loading new messages after the last", async () => {
  mount(thread("refreshing"));

  await expect.element(page.getByText("Store::open takes a clock now")).toBeVisible();
  await expect.element(mark()).toBeVisible();
  expect(document.querySelector(".armada-session-thread__skeleton")).toBeNull();
  expect(list().hasAttribute("aria-busy")).toBe(false);

  const items = [...list().children];
  const last = items.findIndex((one) => one.textContent?.includes("Store::open takes a clock now"));
  const marked = items.findIndex((one) => one.contains(mark().element()));
  expect(last).toBeGreaterThanOrEqual(0);
  expect(marked).toBeGreaterThan(last);
});
