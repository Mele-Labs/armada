// Job detail's two columns, measured through `App` at the window widths the
// owner hit — the defect being that the run column held its 380px, Helm's dock
// held its 380px, and the step panel took what was left, which at 1280 was
// 192px of one-letter-a-line log.
//
// **#1583 took one of those three terms away.** The dock is a layer now, so
// the band this file used to measure — 1101 to 1279, the left column at its
// rail paying for it — is gone, and the claim is the one it did not have
// before: the content is the same width with Helm open and shut.
//
// **A geometry test, so it lives here and not in a story.** The widths that
// matter are decided by the whole window, and a story drawing one composition
// in a sized `div` cannot see any of them. `Layer.test.tsx` is the precedent
// for driving the viewport in this project.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { FLEET_DOT_TONE, fleetSaid } from "@armada/components";
import type { Connection } from "@armada/protocol";
import { SHORT_LABEL } from "@armada/shell";
import { workingAPlan } from "./fixtures/build/index";

import { mount, onJob, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;

// The project's own size, back after every test: the viewport is the one piece
// of state here that outlives an unmount.
const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

/** The floors and the hairline, as `spacing.css` declares them. Read, never retyped. */
function floor(
  token:
    | "--w-run-column-min"
    | "--w-step-panel-min"
    | "--sidebar-default"
    | "--layout-breakpoint"
    | "--sidebar-rail"
    | "--w-sheet"
    | "--border-width",
): number {
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue(token));
  if (!Number.isFinite(value)) throw new Error(`${token} is not declared`);
  return value;
}

const boxOf = (selector: string): DOMRect => {
  const element = document.querySelector<HTMLElement>(selector);
  if (element === null) throw new Error(`${selector} is not drawn`);
  return element.getBoundingClientRect();
};

/**
 * The worst text on the board: the node drawn on the most lines per word it
 * has. **A run of text never needs more lines than it has words** — one line
 * each is the worst honest wrapping — so a ratio above 1 is a word that was
 * broken down the middle, which is the defect this file exists for. Measured
 * with a range over each text node.
 *
 * **Read on the cards since 29 Sep 2026.** It read the step panel, which the
 * Overview reframe deleted — and the panel was where the 192px log broke, so
 * the cards that took its place are where the same defect would show.
 */
function brokenWord(): { text: string; words: number; lines: number } | null {
  const panel = document.querySelector<HTMLElement>(".armada-overview-board__cards");
  if (panel === null) throw new Error("the board's cards are not drawn");
  const walker = document.createTreeWalker(panel, NodeFilter.SHOW_TEXT);
  let worst: { text: string; words: number; lines: number } | null = null;
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const text = (node.textContent ?? "").trim();
    if (text === "") continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    const lines = new Set(
      [...range.getClientRects()].filter((rect) => rect.width > 0 && rect.height > 0).map((rect) => Math.round(rect.top)),
    ).size;
    if (lines === 0) continue;
    const words = text.split(/\s+/).length;
    if (lines <= words) continue;
    if (worst === null || lines - words > worst.lines - worst.words) worst = { text, words, lines };
  }
  return worst;
}



/**
 * App on the Job the owner had open, at `width`, with the layout settled.
 * **Helm is shut**, which is how Bridge opens since #1583; `withHelm` is what
 * puts it up.
 *
 * **It waits on the Overview board.** Until 29 Sep 2026 it waited on the run
 * tree's heading and on `.armada-inside__run`, the two-column arrangement the
 * Overview reframe replaced — a Job detail draws one board of cards now, and
 * the cards are what this file mounts a Job to measure the shell around.
 */
async function atWidth(width: number): Promise<void> {
  await page.viewport(width, 860);
  mount(onJob(workingAPlan()), SLICES);
  await expect.poll(() => document.querySelector(".armada-overview-board__cards") !== null).toBe(true);
}

/**
 * Put Helm up. **`⌘J` rather than the title row's button**: it is the one way
 * in that works at every width and from inside a field, and it is what a
 * person presses.
 */
async function withHelm(): Promise<void> {
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.poll(() => document.querySelector(".armada-shell__dock-layer, .armada-sheet")).not.toBe(null);
}


// 1280 is where the owner found it. It was wide enough for Helm's dock to sit
// **Job detail no longer has two columns.** The run tree and the step panel
// were one arrangement, and the Overview reframe of 29 Sep 2026 replaced them
// with a board of cards — so the 1280 measurement #1428 landed, and the 1512
// one that stopped it reading as "the run column got narrower", are claims
// about a screen that is gone. What survives is everything below: the shell
// around a Job, which is what this file was always really measuring.

// ---- what #1583 claims, measured ------------------------------------------
//
// **Opening Helm changes nothing about the width of what is behind it.** The
// issue's own definition of done, and the fact three screens each wrote their
// own workaround around on 22 September. Measured on the shell's panel rather
// than on job detail's columns, because it is true of every surface and job
// detail is only the one with a fixture open in this file.

const panelWidth = (): number => boxOf(".armada-shell__panel").width;

test.each([2000, 1512, 1440, 1280, 1101])(
  "at %i the content is the same width with Helm shut and open",
  async (width) => {
    await atWidth(width);
    const shut = panelWidth();
    await withHelm();
    expect(panelWidth()).toBe(shut);
    // Shut again, by the one press the issue asks for — the dock's own Close,
    // scoped to it, because job detail draws a Close of its own.
    await page.getByRole("complementary", { name: "Helm" }).getByRole("button", { name: /^Close/ }).click();
    await expect.poll(() => document.querySelector(".armada-shell__dock-layer")).toBe(null);
    expect(panelWidth()).toBe(shut);
  },
);

// Below the breakpoint the dock is a sheet rather than a panel, over the work
// rather than in its flow — so the claim holds there too, and for a different
// reason worth measuring separately.
test.each([1000, 768])("at %i the content is the same width with the sheet shut and open", async (width) => {
  await atWidth(width);
  const shut = panelWidth();
  await withHelm();
  await expect.element(page.getByRole("dialog", { name: "Helm" })).toBeVisible();
  expect(panelWidth()).toBe(shut);
});

// **Where the inspector folded, there is nothing to fold.** Below
// `--layout-breakpoint` the step panel used to become a sheet over the run,
// and six claims here measured it: the run taking the whole content, the
// sheet at `--w-sheet`, flush at 768, Escape closing it, and the log not
// clipping at 1150. The board of cards that replaced both columns on 29 Sep
// 2026 reflows instead of folding, and its own claim is `overview-boards`'.

// ---- the left column, and the band it used to collapse in -----------------
//
// The column had two collapse points: `--layout-breakpoint`, and a second at
// `--window-fold-left` where it fell to its 48px rail so Helm's dock could
// keep its 380. #1435 removed the column outright there and the owner
// corrected it on 18 Sep 2026 to the rail; **#1583 removed the reason**, so
// the band is gone and the rail's 152px are not owed to anyone. What these
// measure is the other half of the same claim: the column is where the window
// puts it, and Helm cannot move it.

const leftColumn = (): Element | null => document.querySelector(".armada-shell__left");

/**
 * The left column's own width — its computed one, not its box. It is
 * `content-box` with `--space-4` of padding either side, so its rect is 32px
 * wider than the width every other rule in the shell means by it.
 */
function leftColumnWidth(): number {
  const element = leftColumn();
  if (element === null) throw new Error("the left column is not drawn");
  return parseFloat(getComputedStyle(element).width);
}

/**
 * What the content owes at a width where it could not pay before.
 *
 * **Two column floors until 29 Sep 2026** — `--w-step-panel-min` and
 * `--w-run-column-min`, which is what the 192px log came down to. The board
 * of cards that replaced both has no floor to meet; what it still owes is the
 * thing the floors were for, which is that no word in it breaks a letter to
 * a line. That is `brokenWord`, and it was always the claim underneath.
 */
function contentHolds(): void {
  expect(brokenWord()).toBe(null);
}

// The old band, end to end, with Helm up. Every one of these drew the rail
// before; all four are the ordinary case now.
test.each([1101, 1150, 1250, 1279])(
  "at %i the left column stands at width with Helm up, and the content holds",
  async (width) => {
    await atWidth(width);
    await withHelm();
    expect(leftColumnWidth()).toBe(floor("--sidebar-default"));
    contentHolds();
  },
);

// One pixel over the breakpoint, which is where the column now goes from rail
// to width. Nothing between here and the top of the range is a special case.
test("one pixel over --layout-breakpoint the column is at width, with Helm up", async () => {
  await atWidth(floor("--layout-breakpoint") + 1);
  await withHelm();
  expect(leftColumnWidth()).toBe(floor("--sidebar-default"));
  contentHolds();
});

// The collapse is the window's arithmetic, so it is on every surface and not
// only the one that needs it. Navigation and Fleet go together and all
// of them are still in the tree, which is the #1435 regression: they were not.
test("at the rail, every panel is there rather than gone", async () => {
  await atWidth(1000);
  await expect.poll(leftColumn).not.toBe(null);
  expect(document.querySelector(".armada-shell__left-handle")).toBe(null);
  // Two nav panels since the owner split Navigation, and both keep their
  // glyphs at 48px — the rail's own form, never an absent column.
  expect(document.querySelectorAll(".armada-sidebar").length).toBe(2);
  await expect.element(page.getByRole("button", { name: "Overview" }).first()).toBeVisible();
  // Fleet keeps one status dot, as a named region.
  await expect.element(page.getByRole("region", { name: "Fleet" }).first()).toBeVisible();
});

// ---- Fleet's dot, in the title row and at the rail ------------------------
//
// Whether the dot is drawn used to be decided by the window's width, which is
// exactly what a story cannot see. It is not decided by anything now — #1438's
// condition was corrected on 18 Sep 2026 and the title row carries it at every
// width — so what is measured here is the pair: the title row's dot, and the
// Fleet panel's own at the rail, which is a second element with the same name.

/** The title row's Fleet dot. Its own class, because the pair is what the rail widths count. */
const fleetDot = (): Element | null => document.querySelector(".armada-title-bar__fleet");

// The mock answers, so the reading is a connected Fleet's. Every part comes
// from the producers Bridge itself uses — `shortLabelOf` for the panel's own
// word, `fleetSaid` for the sentence and `FLEET_DOT_TONE` for the hue — rather
// than retyped here, so a rename fails this rather than passing a stale copy.
const RUNNING = "connected" satisfies Connection["state"];
const SAID = fleetSaid(SHORT_LABEL[RUNNING]);

// At the rail there are two readings of one fact and the owner chose both: the
// title row's permanent dot, and the panel collapsed to its own dot. The second
// was silent until 18 Sep 2026 — an `aria-hidden` mark inside a region named
// "Fleet" — so a person who cannot see the hue was told the panel's name and
// never its state at the one width where the name is all there is.
test.each([1000, 900])("at %i both Fleet dots are named and toned", async (width) => {
  await atWidth(width);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
  // Found by accessible name, never by class: the name is the assertion, and
  // it is what a person who cannot see the hue is left with.
  const named = page.getByRole("img", { name: SAID });
  expect(named.elements()).toHaveLength(2);
  for (const element of named.elements()) expect(element.getAttribute("title")).toBe(SAID);
  expect(fleetDot()?.firstElementChild?.getAttribute("data-tone")).toBe(FLEET_DOT_TONE.running);
  // Liveness only: what the panel's rows carried is not smuggled into either.
  expect(fleetDot()?.textContent).toBe("");
});

test("at 1512 the title row keeps its dot and the panel says it in words instead", async () => {
  await atWidth(1512);
  expect(leftColumnWidth()).toBeGreaterThan(floor("--sidebar-rail"));
  // One, not two: the expanded panel has the word in its body, so it takes no
  // name of its own and the row's dot is the only thing carrying the sentence.
  expect(page.getByRole("img", { name: SAID }).elements()).toHaveLength(1);
  await expect.element(page.getByRole("img", { name: SAID }).first()).toBeVisible();
  // And the facts a dot could not carry are back beside it.
  await expect.element(page.getByText("pid").first()).toBeVisible();
});

// `--layout-breakpoint` is the widest window the rail is drawn at, and one
// pixel over it the column stands at its full width. Two tests and not one,
// because `mount` puts a second App in the document rather than replacing it.
test("at --layout-breakpoint the column is the rail", async () => {
  await atWidth(floor("--layout-breakpoint"));
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
});

test("one pixel over --layout-breakpoint the column is at its full width", async () => {
  await atWidth(floor("--layout-breakpoint") + 1);
  expect(leftColumnWidth()).toBe(floor("--sidebar-default"));
  contentHolds();
});

// **Helm is not one of the terms.** This is the test that used to say the
// opposite — at 1150 the column was the rail until ⌘J gave the pixels back,
// which is the coupling #1583 ends. Both presses, so a width that only holds
// while the dock has never been opened would fail here.
test("at 1150 the left column does not move when Helm opens or closes", async () => {
  await atWidth(1150);
  const width = leftColumnWidth();
  expect(width).toBe(floor("--sidebar-default"));
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.poll(() => document.querySelector(".armada-shell__dock-layer")).not.toBe(null);
  expect(leftColumnWidth()).toBe(width);
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.poll(() => document.querySelector(".armada-shell__dock-layer")).toBe(null);
  expect(leftColumnWidth()).toBe(width);
});

// Above the breakpoint nothing is at the rail, and below it everything is.
test.each([1512, 1440, 1150, 1101])("at %i the left column is at its full width", async (width) => {
  await atWidth(width);
  await expect.poll(leftColumn).not.toBe(null);
  expect(leftColumnWidth()).toBe(floor("--sidebar-default"));
});

test.each([1100, 900, 768])("at %i the left column is still the rail", async (width) => {
  await atWidth(width);
  await expect.poll(leftColumn).not.toBe(null);
  expect(leftColumnWidth()).toBe(floor("--sidebar-rail"));
});
