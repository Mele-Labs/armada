// The Land board on a Job recorded off a real Fleet: Job 2, finished, its pull
// request merged (`recorded/landed-and-merged`, 1 Oct 2026).
//
// **The owner read this board on that Job and left twelve notes.** Every
// sentence he asked about described something Fleet does not send — a count of
// verdicts nobody recorded, a rule for completing on a Job that had completed,
// two test sets nothing had run — and the mock had drawn the board on a draft
// that filled each one in, so none of them read wrong there. This file draws
// it on the wire as Fleet served it, at the width of his laptop.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

/** Open Job 2's Overview, where the board is drawn, at a laptop's width. */
async function landed(width = 1280): Promise<void> {
  await page.viewport(width, 800);
  mount("recorded/landed-and-merged");
  await onScreen();
  await expect.element(page.getByText("Landed", { exact: true })).toBeVisible();
}

/** The badges inside one region, by the word each draws. */
function badgesIn(selector: string): string[] {
  return [...document.querySelectorAll(`${selector} .armada-badge`)].map((one) => one.textContent ?? "");
}

test("nothing on the board describes what Fleet did not send", async () => {
  await landed();
  for (const gone of [
    /No verdict was recorded for/,
    /with a verdict recorded/,
    /Completes when/,
    /run again at handoff/i,
    /not run again before it was offered/,
    /^Run by hand$/,
    /Nobody has run one of these/,
    /no before-run/,
    /Nothing has read what this Job holds/,
    /No Manifest was read for this Job/,
    /has no worktree, so it has no branch/,
    /Nothing times a group/,
    /Dispatch a follow-up/,
  ]) {
    expect(page.getByText(gone).elements(), String(gone)).toHaveLength(0);
  }
  // The one criterion still says what answered it, in its own row.
  await expect.element(page.getByText("no verdict recorded", { exact: true })).toBeVisible();
});

test("the pull request's state is a badge in the header and on the board", async () => {
  await landed();
  expect(badgesIn(".armada-job-head__facts")).toEqual(["Merged"]);
  expect(badgesIn(".armada-land")).toEqual(["Merged"]);
});

test("the act beside the verdict is the registry's Dispatch, with its key", async () => {
  await landed();
  const lead = document.querySelector<HTMLElement>(".armada-land__lead")!;
  const acts = [...lead.querySelectorAll("button")].map((one) => one.textContent);
  // The guide's `?` beside the verb, and the one act: Dispatch and its `n`.
  expect(acts.filter((one) => one?.startsWith("Dispatchn"))).toHaveLength(1);
});

test("no value on the board is cut off at 1280 wide", async () => {
  await landed(1280);
  const drawn = [...document.querySelectorAll<HTMLElement>(".armada-land .armada-outcome__value")];
  // The pull request, the commit, the branch and the record: Job 2 gave its
  // worktree back, so that row is not drawn at all.
  expect(drawn.map((one) => one.textContent)).toEqual([
    expect.stringMatching(/\/NickMele\/armada\/pull\/1750$/),
    "daae5427cfd2c705d4cbcf719121e6b5e1b285d7",
    "armada/2-retire-guides-8-and-20-add-validation-that",
    expect.stringMatching(/\/\.armada\/logs\/2-retire-guides-8-and-20-add-validation-that\.jsonl$/),
  ]);
  for (const one of [
    ...drawn,
    ...document.querySelectorAll<HTMLElement>(".armada-land .armada-outcome__meta"),
  ]) {
    expect(one.scrollWidth, one.textContent ?? "").toBeLessThanOrEqual(one.clientWidth);
    const region = one.closest<HTMLElement>(".armada-outcome")!.getBoundingClientRect();
    expect(one.getBoundingClientRect().right, one.textContent ?? "").toBeLessThanOrEqual(region.right);
  }
});

test("the run's table spans its region rather than bunching at the left", async () => {
  await landed(1280);
  const list = document.querySelector<HTMLElement>(".armada-land .armada-outcome__steps")!;
  const region = list.getBoundingClientRect();
  const took = [...list.querySelectorAll<HTMLElement>(".armada-outcome__run-when")].map(
    (one) => one.getBoundingClientRect().left,
  );
  // The last column starts in the last third of the region, not beside the label.
  expect(Math.min(...took)).toBeGreaterThan(region.left + (region.width * 2) / 3);
});
