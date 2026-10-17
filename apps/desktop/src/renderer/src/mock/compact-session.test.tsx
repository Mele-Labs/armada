// The compact Session view, through `App` on `compact-session-view` at a 1512 by 900 window: what waits on him is the thread's last card and
// nothing is pinned; a chip over the composer jumps back to it; the Flight control keys open Artifacts, Subagents, Checks and Slot over the
// thread and Back returns to the exact spot; the Bays key opens every bay the Session holds, and lights, with a call in the thread, only when one is wrong. The walks
// `compactSessionView` and `compactCallsInThread` play it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const TITLE = "Retry the slot lease";
const work = () => page.getByRole("region", { name: "Work", exact: true });
const thread = () => page.getByRole("region", { name: "Thread", exact: true });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const tile = (name: string) => controls().getByRole("button", { name });
const scroller = () => thread().element() as HTMLElement;
const tail = () => scroller().querySelector<HTMLElement>(".armada-session-thread__tail");

async function open() {
  await page.viewport(1512, 900);
  mount("compact-session-view");
  await onScreen();
  await work().getByRole("button", { name: TITLE, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Session: ${TITLE}` })).toBeVisible();
  await expect.element(thread()).toBeVisible();
}

test("the question is the thread's last card, answered where it stands, and nothing is pinned", async () => {
  await open();
  await expect.poll(() => tail()).not.toBeNull();
  const last = tail()!;
  expect(scroller().contains(last)).toBe(true);
  expect(last.querySelector('[aria-label="Waiting on you"]')).not.toBeNull();
  // It is the last thing in the thread's rows, after the final message.
  const rows = [...scroller().querySelectorAll("ol > li")];
  expect(rows.at(-1)).toBe(last);
  expect(page.getByRole("region", { name: "Pinned calls" }).query()).toBeNull();
  expect(getComputedStyle(scroller()).paddingBlockEnd).toBe("0px");
});

test("what Needs you holds is in the same last card, after the question", async () => {
  await open();
  await expect.poll(() => tail()).not.toBeNull();
  const needs = tail()!.querySelector('[aria-label="Needs you"]');
  expect(needs).not.toBeNull();
  expect(needs!.compareDocumentPosition(tail()!.querySelector('[aria-label="Waiting on you"]')!) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
});

test("scrolled away from its last card, a chip over the composer names the call and jumps back to it", async () => {
  await open();
  const chip = () => page.getByRole("button", { name: "Question" });
  await expect.poll(() => tail()).not.toBeNull();
  expect(chip().query()).toBeNull();
  scroller().scrollTop = 0;
  await expect.element(chip()).toBeVisible();
  await chip().click();
  await expect.poll(() => scroller().scrollHeight - scroller().clientHeight - scroller().scrollTop).toBeLessThan(30);
  await expect.element(chip()).not.toBeInTheDocument();
});

test("the Flight control keys 8, 9 and 0 open Artifacts, Subagents and Checks, and Esc is Back to the thread as it was", async () => {
  await open();
  await expect.poll(() => tail()).not.toBeNull();
  scroller().scrollTop = Math.round(scroller().scrollHeight / 3);
  const at = scroller().scrollTop;
  expect(at).toBeGreaterThan(0);
  for (const [digit, name] of [["8", "Artifacts"], ["9", "Subagents"], ["0", "Checks"]] as const) {
    work().element().focus();
    await userEvent.keyboard(digit);
    await expect.element(page.getByRole("region", { name, exact: true })).toBeVisible();
    // Focus is inside the layer, so the keyboard drives it.
    expect(page.getByRole("region", { name, exact: true }).element().contains(document.activeElement)).toBe(true);
    await userEvent.keyboard("{Escape}");
    await expect.element(page.getByRole("region", { name, exact: true })).not.toBeInTheDocument();
    // The thread underneath was never unmounted, so it is where it was.
    expect(scroller().scrollTop).toBe(at);
  }
});

test("Artifacts reads the Session's pull requests, pages, files, sketches, subagents and commands, and typing narrows them", async () => {
  await open();
  await tile("Artifacts").click();
  const layer = page.getByRole("region", { name: "Artifacts", exact: true });
  for (const group of ["Pull requests", "Commits", "Pages", "Files", "Docs", "Sketches", "Subagents", "Commands"]) {
    await expect.element(layer.getByRole("region", { name: group, exact: true })).toBeVisible();
  }
  await userEvent.fill(layer.getByRole("searchbox", { name: "Search artifacts" }), "caller");
  await expect.element(layer.getByRole("button", { name: "Find every caller of lease_slot" })).toBeVisible();
  await expect.element(layer.getByRole("region", { name: "Pull requests", exact: true })).not.toBeInTheDocument();
});

test("Subagents lists the real subagents with the running ones first", async () => {
  await open();
  await tile("Subagents").click();
  const layer = page.getByRole("region", { name: "Subagents", exact: true });
  const titles = [...layer.element().querySelectorAll(".sv-layer__title-line")].map((one) => one.textContent);
  expect(titles).toEqual(["Mirror the attempt count in the TypeScript wire", "Show the attempt on the bay tile", "Find every caller of lease_slot"]);
});

test("Checks lists the runs the Session made, newest first", async () => {
  await open();
  await tile("Checks").click();
  const layer = page.getByRole("region", { name: "Checks", exact: true });
  await expect.element(layer.getByRole("button", { name: "desktop, running" })).toBeVisible();
  const names = [...layer.element().querySelectorAll("[data-layer-row]")].map((one) => one.getAttribute("aria-label"));
  expect(names).toEqual(["desktop, running", "fleet, passed"]);
});

test("the Bays key is lit only when a bay is wrong, and then the thread has the call and the key opens every bay", async () => {
  await open();
  await expect.element(tile("Bays")).not.toHaveAttribute("aria-disabled");
  expect(tile("Bays").element().hasAttribute("data-lit")).toBe(false);
  timePasses();
  await expect.poll(() => tile("Bays").element().hasAttribute("data-lit")).toBe(true);
  // The call is in the thread's last card, with the others.
  await expect.poll(() => tail()?.textContent ?? "").toContain("Slot 3 is stranded");
  await tile("Bays").click();
  const layer = page.getByRole("region", { name: "Bays", exact: true });
  await expect.element(layer.getByRole("region", { name: "slot-3" }).getByRole("button", { name: "Show in Bays" })).toBeVisible();
  expect(layer.element().textContent).toContain("Stranded");
  expect(layer.element().textContent).toContain("31 commits");
});

test("a write clash lights the Bays key and raises no call in the thread", async () => {
  await page.viewport(1512, 900);
  mount("compact-slot-clash");
  await onScreen();
  await work().getByRole("button", { name: TITLE, exact: true }).click();
  await expect.poll(() => tile("Bays").element().hasAttribute("data-lit")).toBe(true);
  await expect.poll(() => tail()).not.toBeNull();
  expect(tail()!.textContent).not.toContain("conflicts with another holder");
});

test("hovering a clash's other Session in Bays lights its row in Work", async () => {
  await page.viewport(1512, 900);
  mount("compact-slot-clash");
  await onScreen();
  await work().getByRole("button", { name: TITLE, exact: true }).click();
  await tile("Bays").click();
  const row = () => document.querySelector<HTMLElement>('[data-row="session:compact-4"]');
  expect(row()?.hasAttribute("data-hover")).toBe(false);
  await page.getByRole("region", { name: "Bays", exact: true }).getByRole("button", { name: "Open Rotate the audit log" }).hover();
  await expect.poll(() => row()?.hasAttribute("data-hover")).toBe(true);
});

test("a Session calling him pulses its row in Work, and the one that is open does not", async () => {
  await open();
  const calling = () => [...document.querySelectorAll<HTMLElement>("[data-row][data-calling]")].map((one) => one.getAttribute("aria-label"));
  await expect.poll(calling).toEqual(expect.arrayContaining(["Prune the stale branches", "Name the store migrations"]));
  expect(calling()).not.toContain(TITLE);
  expect(document.querySelector(".deck-incoming")).toBeNull();
});
