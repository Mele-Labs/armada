// The compact Session view, through `App` on `compact-session-view` at a 1512 by 900 window: what waits on him is the Needs you panel on the
// composer, the question its first row, and nothing is pinned or left at the thread's tail; no chip floats over the composer for a question; the
// Flight control keys open Artifacts, Subagents, Checks and Slot over the thread and Back returns to the exact spot; the Bays key opens every bay
// the Session holds, and lights, with a call in the Needs you list, only when one is wrong. The walks `compactSessionView` and
// `compactCallsInThread` play it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const TITLE = "Retry the slot lease";
const work = () => page.getByRole("region", { name: "Jobs", exact: true });
const thread = () => page.getByRole("region", { name: "Thread", exact: true });
// The four views of a Job are keys in its head, and no longer tiles of the Flight controls.
const tile = (name: string) => page.getByRole("region", { name: /^Job: / }).getByRole("button", { name, exact: true });
const scroller = () => thread().element() as HTMLElement;
const tail = () => scroller().querySelector<HTMLElement>(".armada-session-thread__tail");
const needs = () => page.getByRole("region", { name: "Needs you", exact: true });
// The rows the Needs you list reads, in order. The panel is open on its list once a question has arrived.
const listed = () => [...needs().element().querySelectorAll(".armada-needs-panel__list button")].map((one) => one.getAttribute("aria-label"));

async function open() {
  await page.viewport(1512, 900);
  mount("compact-session-view");
  await onScreen();
  await work().getByRole("button", { name: TITLE, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Job: ${TITLE}` })).toBeVisible();
  await expect.element(thread()).toBeVisible();
}

test("the question is the first row of the Needs you list, not a card in the thread, and nothing is pinned", async () => {
  await open();
  await expect.element(needs().getByRole("button", { name: /^Open / }).first()).toBeVisible();
  expect(tail()).toBeNull();
  expect(scroller().querySelector('[aria-label="Waiting on you"]')).toBeNull();
  // Pressing its row opens the question to be answered where it is asked.
  await needs().getByRole("button", { name: /^Open / }).first().click();
  await expect.element(needs().getByRole("article", { name: "Waiting on you" })).toBeVisible();
  expect(page.getByRole("region", { name: "Pinned calls" }).query()).toBeNull();
  expect(getComputedStyle(scroller()).paddingBlockEnd).toBe("0px");
});

test("what Needs you holds is in the same list, after the question", async () => {
  await open();
  await expect.element(needs().getByRole("button", { name: /^Open / }).first()).toBeVisible();
  const names = listed();
  expect(names.length).toBeGreaterThan(2);
  expect(names[0]).toContain("How many attempts before it gives up?");
  expect(names[1]).toContain("The retry reads the pool without holding the lease lock");
  expect(names).toContain("Open Approve #2390");
});

test("scrolled away from the end of the thread, no chip floats over the composer for the question", async () => {
  await open();
  await expect.element(needs().getByRole("button", { name: /^Open / }).first()).toBeVisible();
  scroller().scrollTop = 0;
  await new Promise((done) => setTimeout(done, 300));
  expect(page.getByRole("button", { name: "Question" }).query()).toBeNull();
  expect(document.querySelector(".sv-callchip__press")).toBeNull();
});

test("the Flight control keys ⌥⇧8, ⌥⇧9 and ⌥⇧0 open Artifacts, Subagents and Checks, and Esc is Back to the thread as it was", async () => {
  await open();
  await expect.poll(() => scroller().scrollHeight - scroller().clientHeight).toBeGreaterThan(0);
  scroller().scrollTop = Math.round(scroller().scrollHeight / 3);
  const at = scroller().scrollTop;
  expect(at).toBeGreaterThan(0);
  for (const [digit, name] of [["8", "Artifacts"], ["9", "Subagents"], ["0", "Checks"]] as const) {
    work().element().focus();
    await userEvent.keyboard(`{Alt>}{Shift>}${digit}{/Shift}{/Alt}`);
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
  await expect.element(layer).toBeVisible();
  const titles = [...layer.element().querySelectorAll(".sv-ledger__title")].map((one) => one.textContent);
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

test("the Bays key is lit only when a bay is wrong, and then the Needs you list has the call and the key opens every bay", async () => {
  await open();
  await expect.element(tile("Bays")).not.toHaveAttribute("aria-disabled");
  expect(tile("Bays").element().hasAttribute("data-lit")).toBe(false);
  timePasses();
  await expect.poll(() => tile("Bays").element().hasAttribute("data-lit")).toBe(true);
  // The call is a row of the Needs you list, with the others.
  await expect.poll(() => listed()).toContain("Open Slot 3 is stranded");
  await tile("Bays").click();
  const layer = page.getByRole("region", { name: "Bays", exact: true });
  await expect.element(layer.getByRole("button", { name: "Show slot-3 in Bays" })).toBeVisible();
  await expect.element(layer.getByRole("img", { name: "Stranded" })).toBeInTheDocument();
  await expect.element(layer.getByRole("img", { name: "31 behind main" })).toBeInTheDocument();
});

test("a write clash is a lamp in the Bays key's corner that keeps its name, leaves the key unlit and raises no call in the Needs you list", async () => {
  await page.viewport(1512, 900);
  mount("compact-slot-clash");
  await onScreen();
  await work().getByRole("button", { name: TITLE, exact: true }).click();
  await expect.element(tile("Bays").getByRole("img", { name: "Write clash with Name the store migrations, Rotate the audit log" })).toBeVisible();
  // A lamp and not a glyph: no words, smaller than the key, and in its lower right corner clear of the glyph.
  const key = tile("Bays").element().getBoundingClientRect();
  const lamp = tile("Bays").getByRole("img", { name: /^Write clash with/ }).element();
  const at = lamp.getBoundingClientRect();
  expect(lamp.textContent).toBe("");
  expect(at.width).toBeLessThan(key.width / 2);
  expect(at.left + at.width / 2).toBeGreaterThan(key.left + key.width / 2);
  expect(at.top + at.height / 2).toBeGreaterThan(key.top + key.height / 2);
  expect(tile("Bays").element().hasAttribute("data-lit")).toBe(false);
  await expect.element(needs().getByRole("button", { name: /^Open / }).first()).toBeVisible();
  expect(listed().join("\n")).not.toContain("conflicts with another holder");
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
