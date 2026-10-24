// Ending a Job leaves at once. Whichever act ends it (the head's More menu, the done band, a row's menu) goes through one wrapper over
// the Sessions draft, which tells the stage; the stage clears its selection in that press and plays the pane out as a copy that
// folds into its row. Fleet's answer is never waited for: here End is a close that does nothing, so only the press can clear the stage.
// Through `App` over the `session-all-done` scenario. The browser project runs with reduced motion; `motion()` turns it on for the fold.

import { expect, onTestFinished, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { Scenario } from "./scenario";
import { s207SessionAllDone } from "./scenarios/session-all-done";
import { motion, mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The scenario with End that never answers and never ends the Job, so nothing but the press can have cleared the stage. */
const unanswered: Scenario = {
  ...s207SessionAllDone,
  draft: {
    sessions: (board) => {
      const draft = s207SessionAllDone.draft!.sessions!(board);
      return Object.assign(Object.create(draft) as typeof draft, { close: () => undefined });
    },
  },
};

const job = () => page.getByRole("region", { name: /^Job: / });
/** The stage's own pane: the Job's, or the timeline's with none open. A copy leaving is in a `.deck-ghost`, not here. */
const stagePane = () => document.querySelector(".deck-stage > .deck-pane")?.getAttribute("aria-label") ?? "";
const ghost = () => document.querySelector<HTMLElement>(".deck-ghost");
/** Every copy laid over the stage from now on, kept even after it is gone: with reduced motion the tokens are 0ms and a copy would be removed within a frame. */
function laidCopies() {
  const laid: HTMLElement[] = [];
  const watch = new MutationObserver((changes) => {
    for (const one of changes) for (const added of one.addedNodes) if (added instanceof HTMLElement && added.classList.contains("deck-ghost")) laid.push(added);
  });
  watch.observe(document.body, { childList: true, subtree: true });
  onTestFinished(() => watch.disconnect());
  return laid;
}

async function open(title: string) {
  mount(unanswered);
  await onScreen();
  await toSession(title);
  expect(stagePane()).toContain("Job: ");
}

const endFromHead = async () => {
  await userEvent.click(job().getByRole("button", { name: "More" }));
  await userEvent.click(page.getByRole("menuitem", { name: "End job" }));
};

const endFromBand = () => userEvent.click(page.getByRole("region", { name: "Nothing left to do" }).getByRole("button", { name: "End job" }));

const endFromRow = async (title: string) => {
  const row = page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: title, exact: true }).element();
  const box = row.getBoundingClientRect();
  row.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, view: window, clientX: box.x + 4, clientY: box.y + 4, button: 2 }));
  await userEvent.click(page.getByRole("menuitem", { name: "Close" }));
};

test("End from the Job head's More menu clears the stage at once, without Fleet's answer", async () => {
  await open("Release notes script");
  await endFromHead();
  // Not polled: the very next render, with the Job still not ended.
  expect(stagePane()).not.toContain("Job: ");
  expect(stagePane()).not.toBe("");
});

test("End from the done band clears the stage at once, without Fleet's answer", async () => {
  await open("Pin the store clock");
  await endFromBand();
  expect(stagePane()).not.toContain("Job: ");
  expect(stagePane()).not.toBe("");
});

test("End from a row's menu on the open Job clears the stage at once, without Fleet's answer", async () => {
  await open("Release notes script");
  await endFromRow("Release notes script");
  expect(stagePane()).not.toContain("Job: ");
  expect(stagePane()).not.toBe("");
});

test("with motion the pane leaves as a copy over the stage that animates and is then gone", async () => {
  await motion();
  await open("Release notes script");
  const laid = laidCopies();
  await endFromHead();
  expect(stagePane()).not.toContain("Job: ");
  expect(laid).toHaveLength(1);
  expect(laid[0]!.closest(".deck-stage")).not.toBeNull();
  expect(laid[0]!.querySelector('[aria-label^="Job: "]')).not.toBeNull();
  await expect.poll(() => ghost()?.getAnimations().length ?? 0).toBeGreaterThan(0);
  await expect.poll(ghost).toBeNull();
});

test("with reduced motion no copy is laid over the stage, and the selection is still cleared", async () => {
  await open("Release notes script");
  const laid = laidCopies();
  await endFromHead();
  expect(stagePane()).not.toContain("Job: ");
  // A copy is laid in the press and started a frame later; give it both.
  await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  expect(laid).toHaveLength(0);
});
