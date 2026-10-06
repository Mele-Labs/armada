// The band over Pulse, through `App`.
//
// The owner, 28 September 2026: *"this label/value pairs are very difficult to
// scan. I should be able to quickly scan through them."* The design board had
// answered it — `Pulse · the destination` — and the answer was flattened on
// the way in, so these claims are what the board specifies, read off the
// window rather than off the component.
//
// **Measured, not read.** Which of a label and its figure is larger is
// geometry: a figure set at the label's size reads identically through
// `getByRole`, and that is the shape of the defect this replaces.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { GUIDES, GUIDE_PULSE, GUIDE_WORKTREE_SIZE } from "@armada/components";

import { running } from "@armada/jobs/fixtures/build/index";

import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** What `guidance.tsx` writes. `guides-met.ts` fills it; two tests here empty it. */
const KEY = "armada.bridge.guides";

/** Open Pulse the way a person reaches it, by the strip. */
async function onPulse(): Promise<void> {
  await page.getByRole("tab", { name: "Pulse" }).click();
  await expect.element(page.getByRole("tabpanel", { name: "Pulse" })).toBeVisible();
}

/** One figure on the band: its label, its reading and the cap under it. */
function figure(label: string): { label: HTMLElement; reading: HTMLElement; cap: HTMLElement | null } {
  const term = [...document.querySelectorAll<HTMLElement>(".armada-figures[data-column='strip'] dt")].find(
    (one) => one.textContent === label,
  );
  if (term === undefined) throw new Error(`no figure on the band reads \`${label}\``);
  const row = term.parentElement!;
  return {
    label: term,
    reading: row.querySelector<HTMLElement>(".armada-figures__reading")!,
    cap: row.querySelector<HTMLElement>(".armada-figures__detail"),
  };
}

const size = (one: HTMLElement) => Number.parseFloat(getComputedStyle(one).fontSize);

test("the label carries the verb and the figure is a bare number, so the band is numbers to scan", async () => {
  mount(onJob(running()));
  await onPulse();

  // `Judges` over `none out` put the phrase where the reading belongs. The
  // count under `Judges running` is a count, and `0` is no gap: the label
  // already says what is being counted.
  for (const label of ["Drones running", "Checks running", "Judges running", "Processes"]) {
    expect(figure(label).reading.textContent).toMatch(/^\d+$/);
  }
  expect(document.body.textContent).not.toContain("none running");
  expect(document.body.textContent).not.toContain("none out");
});

test("a figure is drawn larger than its label and its label is the dimmer of the two", async () => {
  mount(onJob(running()));
  await onPulse();

  const spend = figure("Spend");
  // Two steps up and one step down, which is the whole of the contrast: the
  // design contract buys emphasis with size and hue and never with weight.
  expect(size(spend.reading)).toBeGreaterThan(size(spend.label));
  expect(getComputedStyle(spend.label).color).not.toBe(getComputedStyle(spend.reading).color);
  expect(Number(getComputedStyle(spend.reading).fontWeight)).toBe(
    Number(getComputedStyle(spend.label).fontWeight),
  );
});

test("the cap sits under the figure on its own line rather than beside it", async () => {
  mount(onJob(running()));
  await onPulse();

  const spend = figure("Spend");
  expect(spend.reading.textContent).toMatch(/^~?\$/);
  expect(spend.reading.textContent).not.toContain(" of ");
  expect(spend.cap?.textContent).toMatch(/^of \$/);
  // Under it, which is what stops the pair reading as one phrase.
  expect(spend.cap!.getBoundingClientRect().top).toBeGreaterThanOrEqual(
    spend.reading.getBoundingClientRect().bottom,
  );
});

test("one rule falls between what the Job is running and what it is taking, and nowhere else", async () => {
  mount(onJob(running()));
  await onPulse();

  const apart = [...document.querySelectorAll<HTMLElement>(".armada-figures__row[data-apart]")];
  expect(apart.map((one) => one.querySelector("dt")?.textContent)).toEqual(["Spend"]);
  // A rule and not a gap: the edge is drawn, and it is the subtle one.
  const edge = getComputedStyle(apart[0]!).borderInlineStartWidth;
  expect(Number.parseFloat(edge)).toBeGreaterThan(0);
  expect(figure("Judges running").label.closest(".armada-figures__row")).not.toHaveAttribute("data-apart");
});

test("the board's own name carries a `?`, and it opens what Pulse is", async () => {
  mount(onJob(running()));
  await onPulse();

  await page
    .getByRole("button", { name: `Open guide ${GUIDE_PULSE.number}, ${GUIDE_PULSE.title}` })
    .click();
  await expect
    .element(page.getByRole("dialog", { name: `Guide ${GUIDE_PULSE.number}, ${GUIDE_PULSE.title}` }))
    .toBeVisible();
});

/**
 * The owner's own window: he had been here before, so every piece but this one
 * was met, and arriving on Pulse handed him *How much disk is this using?* —
 * one fact inside the tab rather than what the tab is.
 *
 * **Met everything but Pulse, rather than nothing.** A window that has met
 * nothing spends the session's one self-opened card on whatever it meets
 * first, which would make this pass for the wrong reason.
 *
 * **Raised on the Job, not on the tab, since 29 Sep 2026.** Overview draws a
 * Pulse card and the card carries `GUIDE_PULSE`, so the piece is met on
 * arrival at the Job — a person meets what Pulse is beside the reading that
 * sent them there, one press before the destination. The second half of the
 * claim is untouched: the guide about disk is a fact inside Pulse and is
 * never what a person is handed on the way in.
 */
test("the Pulse card raises what Pulse is, and never the guide about disk", async () => {
  window.localStorage.setItem(
    KEY,
    JSON.stringify({
      off: false,
      cardSeen: true,
      met: GUIDES.filter((one) => one.piece !== GUIDE_PULSE.piece).map((one) => one.piece),
    }),
  );
  mount(onJob(running()));

  await expect
    .element(page.getByRole("dialog", { name: `Guide ${GUIDE_PULSE.number}, ${GUIDE_PULSE.title}` }))
    .toBeVisible();
  expect(
    page.getByRole("dialog", {
      name: `Guide ${GUIDE_WORKTREE_SIZE.number}, ${GUIDE_WORKTREE_SIZE.title}`,
    }).query(),
  ).toBeNull();
});

/**
 * The owner, 29 Sep 2026: Spend and Turns are read against caps Settings
 * changes, so pressing one goes there. **To the row, not the tab** — a press
 * that left a person at the top of Settings would make them find the cap again.
 */
test("pressing Spend lands on Settings with the cost cap's row in view and its Raise focused", async () => {
  mount(onJob(running()));
  await onPulse();

  const spend = page.getByRole("tabpanel", { name: "Pulse" }).getByRole("button", { name: /^~?\$/ });
  // A bare figure names what pressing it does on hover, and nowhere on screen.
  await expect.element(spend).toHaveAccessibleDescription("Change the cost cap in Settings");
  await spend.click();

  await expect.element(page.getByRole("tab", { name: /^Settings/ })).toHaveAttribute("aria-selected", "true");
  const raise = page.getByRole("button", { name: "Raise the cost cap" });
  await expect.element(raise).toBeInViewport();
  await expect.element(raise).toHaveFocus();
});
