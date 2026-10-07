// The guidance system through `App` — #1602, #1603. Three of its claims are
// about the window rather than about a component, so nothing in Storybook can
// make them: what happens the first time a person meets a piece, what survives
// the window closing, and whether the switch is findable afterwards.
//
// The owner, 23 September 2026: *"Any hints or guides should be something I
// choose to see."* The one card that arrives unasked is paid for by the switch
// arriving with it.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { running } from "@armada/jobs/fixtures/build/index";
import { GUIDES, GUIDE_WORKFLOW } from "@armada/components";

import { onJob } from "./scenario";
import { mountApp } from "./mount";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** What `guidance.tsx` writes. Cleared either side, so no test inherits a memory. */
const KEY = "armada.bridge.guides";

/** What `guide-list-width.ts` writes — the catalogue list's own remembered width. */
const WIDTH_KEY = "armada.bridge.guide-list-width";

function forget(): void {
  window.localStorage.removeItem(KEY);
  window.localStorage.removeItem(WIDTH_KEY);
}

beforeEach(forget);
afterEach(forget);

/** The card for one guide, wherever the provider drew it. */
const cardFor = (guide: { number: number; title: string }) =>
  page.getByRole("dialog", { name: `Guide ${guide.number}, ${guide.title}` });

/**
 * The Work lane's head carries the first mark a person meets on a Job — the
 * Workflow card's, until the canvas became the whole Overview (4 Oct 2026).
 *
 * **It was the run band's step bar until 29 Sep 2026** — `GUIDE_STEP_BAR`,
 * guide 8, whose only mark was in `InsideAJob`. The Overview reframe took
 * that arrangement off, so guide 8 has no piece on screen and cannot be the
 * first one met; the board's own cards carry the marks now.
 */
const runMark = () =>
  page.getByRole("button", { name: `Open guide ${GUIDE_WORKFLOW.number}, ${GUIDE_WORKFLOW.title}` });

const card = () => cardFor(GUIDE_WORKFLOW);
const closeCard = () => card().getByRole("button", { name: "Close" }).click();

test("the first piece a person meets opens its card by itself, and the card carries the switch", async () => {
  mount(onJob(running()));
  await expect.element(card()).toBeVisible();
  await expect
    .element(card().getByRole("switch", { name: /Open a guide the first time/ }))
    .toBeChecked();
});

test("a piece opens itself once and never again, across the window reopening", async () => {
  // Its own host and its own teardown: this test reopens the window, and
  // `unmountAfterEach` would take the first root down a second time.
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const first = mountApp(onJob(running()), host);
  await expect.element(card()).toBeVisible();
  await closeCard();
  first.unmount();
  host.remove();

  // The memory is all that survives a reopen, and a card that came back would
  // be the uninvited thing arriving twice.
  mount(onJob(running()));
  await expect.element(runMark()).toBeVisible();
  expect(card().query()).toBeNull();
});

test("the mark still opens the card after the piece has been met", async () => {
  mount(onJob(running()));
  await closeCard();

  // Pressed at the mark itself: the run opens on where the Job is, below the lane's head.
  (runMark().element() as HTMLElement).click();
  await expect.element(card()).toBeVisible();
  // No switch: the offer was made once, and Settings is where it lives now.
  expect(card().getByRole("switch").query()).toBeNull();
});

test("the switch on the first card turns all of them off, and Settings is where it is found", async () => {
  mount(onJob(running()));
  // The label, not the input: the switch's own text sits over its checkbox,
  // and a label click is what a person's press is anyway.
  await card().getByText("Open a guide the first time I meet a piece").click();
  await closeCard();

  await page.getByRole("button", { name: "Settings", exact: true }).first().click();
  const setting = page.getByRole("switch", { name: /Open a guide the first time/ });
  await expect.element(setting).toBeVisible();
  await expect.element(setting).not.toBeChecked();
});

test("the rail reaches the catalogue, every guide is in the list, and one is open", async () => {
  mount(onJob(running()));
  await closeCard();

  await page.getByRole("button", { name: "Guides", exact: true }).first().click();

  // The catalogue draws a row per guide and opens one beside them, so a guide
  // is reachable rather than already on screen — the list is the claim.
  for (const guide of GUIDES) {
    await expect.element(page.getByRole("button", { name: guide.title })).toBeVisible();
  }

  // Arriving on an empty panel is the thing the list-and-panel arrangement
  // must never do, so the first guide is open before anything is pressed.
  const [first] = GUIDES;
  if (first === undefined) throw new Error("no guides to draw");
  await expect.element(page.getByRole("heading", { name: first.title })).toBeVisible();

  const last = GUIDES[GUIDES.length - 1];
  if (last === undefined) throw new Error("no guides to draw");
  await page.getByRole("button", { name: last.title }).first().click();
  await expect.element(page.getByRole("heading", { name: last.title })).toBeVisible();
});

/**
 * The owner, 25 September 2026: *"Increase the default width of this panel by
 * 10%. Also it would be nice if it was resizable."* The width the list rests
 * at is a token, so it is read off `--w-guide-list` rather than typed here — a
 * test restating the number would pass against its own copy of it.
 */
function resting(): number {
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--w-guide-list"));
  if (!Number.isFinite(value)) throw new Error("--w-guide-list is not declared");
  return value;
}

const handle = () => page.getByRole("separator", { name: "Resize the list of guides" });

/** The column a drag moves, which is what a reader watching this would see change. */
const listWidth = () =>
  Math.round(document.querySelector(".armada-guides__list")?.getBoundingClientRect().width ?? 0);

/**
 * The catalogue, from a window that has just opened. The uninvited card is
 * over the rail the first time a piece is met and never again.
 *
 * **Whichever card is up, rather than a card the caller named.** A Job drew
 * one mark until 29 Sep 2026 and the board of cards that replaced it draws
 * several, so which piece a window meets first is the board's business — and
 * an unclosed card's scrim is what swallows the press to `Guides`.
 */
const openCatalogue = async (met = false) => {
  if (!met) {
    const open = page.getByRole("dialog", { name: /^Guide \d+, / });
    await expect.element(open).toBeVisible();
    await open.getByRole("button", { name: "Close" }).click();
    await expect.poll(() => open.query()).toBeNull();
  }
  await page.getByRole("button", { name: "Guides", exact: true }).first().click();
  await expect.element(handle()).toBeVisible();
};

test("the list rests at its token's width, and its inner edge moves it", async () => {
  mount(onJob(running()));
  await openCatalogue();

  await expect.element(handle()).toHaveAttribute("aria-valuenow", String(resting()));
  expect(listWidth()).toBe(resting());

  await handle().click();
  await userEvent.keyboard("{ArrowRight}");
  await expect.poll(listWidth).toBeGreaterThan(resting());
  // Home is the floor, and the column goes there rather than to whatever the
  // keys had reached — the clamp is the same one a drag reads.
  await userEvent.keyboard("{Home}");
  await expect.poll(listWidth).toBe(Number(handle().element().getAttribute("aria-valuemin")));
});

test("the width is remembered, so the catalogue opens at it next time", async () => {
  // Its own host and its own teardown, `left-column-rail.test.tsx`'s pattern:
  // this test reopens the window, and `unmountAfterEach` would take the first
  // root down a second time.
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const first = mountApp(onJob(running()), host);
  await openCatalogue();
  await handle().click();
  await userEvent.keyboard("{End}");
  const widest = handle().element().getAttribute("aria-valuemax");
  expect(window.localStorage.getItem(WIDTH_KEY)).toBe(widest);

  // A remount is the window reopening: the state is gone and only what was
  // written survives. A column that forgets a drag on every launch is the
  // control not being worth having.
  first.unmount();
  host.remove();

  // **A second window still meets a card**, because the board draws a mark
  // per card and only the first of them was met above. It was one mark on a
  // Job until 29 Sep 2026, and `met` was true here for that reason.
  mount(onJob(running()));
  await openCatalogue();
  await expect.element(handle()).toHaveAttribute("aria-valuenow", widest);
});
