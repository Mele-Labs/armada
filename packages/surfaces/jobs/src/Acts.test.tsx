// Only the pressed act on this header shows it is working — the split
// button's face marks whichever of its own acts is out, and every other
// press on the header is off with no mark. #1117.
//
// # Why the face marks a menu entry too
//
// The menu that opened `kill_job` has already closed by the time Fleet
// answers — a split button is one control once it has, and the face is the
// one surface left standing to say which of its acts is still in flight.
// `SplitButton.stories.tsx` pins the primitive on its own terms; this pins
// that `Acts` hands it the right name.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import type { JobSummary } from "@armada/protocol";
import { Acts } from "./Acts";
import { motion, mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);
// The kill here is named for its hold, and reduced motion — which this project runs under — offers
// none. These claims are about the marks on the header a person without the preference sees.
beforeEach(motion);

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    assigned_drone: "01M1D0X0000016YK5S0JXBXDQ5",
    ...over,
  };
}

function acting(
  summary: JobSummary,
  actingAct?: Parameters<typeof Acts>[0]["actingAct"],
  answered?: Parameters<typeof Acts>[0]["answered"],
): void {
  mount(
    <Acts
      job={summary}
      whole={null}
      render="working"
      acting={actingAct !== undefined}
      actingAct={actingAct}
      answered={answered}
      approving={false}
      stale={false}
      onAct={() => {}}
      onActHeld={() => {}}
      onApprove={() => {}}
      onReport={async () => ({ ok: true })}
      reporting={false}
      onReporting={() => {}}
      onRaiseCap={() => {}}
      raising={false}
      onRaising={() => {}}
      onRaiseTurnCap={() => {}}
      raisingTurns={false}
      onRaisingTurns={() => {}}
      onCopied={() => {}}
    />,
  );
}

// A running Job with a drone offers `kill_job` alone: ending the Drone and
// leaving the Job open is the Drones sheet's act, never this header's (owner,
// 29 Sep 2026).

test("a Job with a drone is offered the Job's kill, and not the Drone's", async () => {
  acting(job());
  await expect.element(page.getByRole("button", { name: "Hold to kill job" })).toBeEnabled();
  await expect.element(page.getByRole("button", { name: "Hold to kill drone" })).not.toBeInTheDocument();
});

test("a kill of the Drone sent elsewhere leaves the header's kill unmarked", async () => {
  acting(job(), "kill_drone");
  const button = page.getByRole("button", { name: "Hold to kill job" });
  await expect.element(button).not.toHaveAttribute("aria-busy");
});

// A Job that is over offers `reclaim_worktree` (lead) and `forget_job`
// (menu), which draws the split button.

const OVER: Partial<JobSummary> = { status: "killed", assigned_drone: undefined };

test("the split button's face is busy for its own lead act", async () => {
  acting(job(OVER), "reclaim_worktree");
  const face = page.getByRole("button", { name: "Reclaiming the worktree…" });
  await expect.element(face).toHaveAttribute("aria-busy", "true");
});

test("the split button's face marks a menu act too, once its menu has closed", async () => {
  acting(job(OVER), "forget_job");
  const face = page.getByRole("button", { name: "Deleting the record…" });
  await expect.element(face).toHaveAttribute("aria-busy", "true");
});

test("a different act leaves the face disabled and unmarked", async () => {
  // `redirect` is `StepActs.tsx`'s own act — never one this header opens —
  // so nothing here reads it as this control's own press.
  acting(job(OVER), "redirect");
  const face = page.getByRole("button", { name: "Reclaim worktree" });
  await expect.element(face).toBeDisabled();
  await expect.element(face).not.toHaveAttribute("aria-busy");
});

// A running Job with no drone offers only `kill_job`, which draws a plain
// button rather than a split control — `Acts.tsx`'s own "nothing in the
// menu is a button" rule.

test("a lone act draws a plain button, busy for its own press", async () => {
  acting(job({ assigned_drone: undefined }), "kill_job");
  const button = page.getByRole("button", { name: "Killing job…" });
  await expect.element(button).toHaveAttribute("aria-busy", "true");
});

test("a lone act's button is unmarked and enabled with nothing out", async () => {
  acting(job({ assigned_drone: undefined }));
  const button = page.getByRole("button", { name: "Hold to kill job" });
  await expect.element(button).not.toHaveAttribute("aria-busy");
  await expect.element(button).toBeEnabled();
});

// The face answers for every act this header sends, a menu entry's included —
// the menu has closed by the time Fleet answers, as with `pending` above.

test("a refused act from the menu answers on the face", async () => {
  acting(job(OVER), undefined, { act: "forget_job", answer: "refused" });
  await expect
    .element(page.getByRole("button", { name: "Reclaim worktree" }))
    .toHaveAttribute("data-answer", "refused");
});

test("an answer to the Drone's kill is not the header's to mark", async () => {
  acting(job(), undefined, { act: "kill_drone", answer: "accepted" });
  await expect
    .element(page.getByRole("button", { name: "Hold to kill job" }))
    .not.toHaveAttribute("data-answer");
});

test("an accepted lone kill answers accepted on its own button", async () => {
  acting(job({ assigned_drone: undefined }), undefined, { act: "kill_job", answer: "accepted" });
  await expect
    .element(page.getByRole("button", { name: "Hold to kill job" }))
    .toHaveAttribute("data-answer", "accepted");
});

test("an answer to another region's act leaves the face unmarked", async () => {
  acting(job(OVER), undefined, { act: "redirect", answer: "refused" });
  await expect
    .element(page.getByRole("button", { name: "Reclaim worktree" }))
    .not.toHaveAttribute("data-answer");
});
