// A killed Job whose Judge refusal was still waiting, through `App`. The owner
// killed Job 1 on 1 Oct 2026 while its plan step's refusal waited on him, and
// Overview went on leading with *A Judge refused 1 of 1 criterion* and an
// `Answer it` button, under a Killed badge. **A Job that is over answers
// nothing**, so its lead offers nothing that would answer it — whatever Fleet
// still serves about the question.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import type { StepDetail } from "@armada/protocol";
import { killed } from "@armada/jobs/fixtures/build/index";
import { freshStep, watchedRead } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const ASKED_AT = "2026-10-01T18:36:51.495Z";
const EXPECTED =
  "Plan describing structural cleanup (dead code removal, validation rules) without removing " +
  "user-facing content from the guides catalogue";
const PRODUCED =
  '"Remove guide 8 from the catalogue" is T1, explicitly removing the guide from the GUIDES list ' +
  "where users can see it";
const CONSEQUENCE =
  "Users looking at available guides will see a different catalogue than before the change, " +
  "altering what is observable and selectable";

/** `plan`, as `GET /jobs/1` served it after the kill: still at the person, refused once. */
function planRefused(): StepDetail {
  return {
    ...freshStep("plan", "Scope the refactor", 0),
    state: "awaiting_human",
    checks: [{ kind: "plan_recorded" }],
    judge_checks: [{ criteria: 1, gaming_check: false }],
    advance_gate: "auto_if_judge_passes",
    check_runs: [{ attempt: 1, name: "plan_recorded", outcome: "passed" }],
    delivers: false,
    attempts: [
      {
        attempt: 1,
        outcome: "awaiting_human",
        started_at: "2026-10-01T18:34:11.365Z",
        ended_at: "2026-10-01T18:36:51.494Z",
      },
    ],
    judged: [
      {
        attempt: 1,
        criterion_id: "structure_only",
        verdict: "not_met",
        expected: EXPECTED,
        produced: PRODUCED,
        consequence: CONSEQUENCE,
      },
    ],
  };
}

/** The owner's Job 1, killed, in the shape Fleet served it. */
function killedWhileRefused(): JobFixture {
  const fixture = killed();
  if (fixture.watched.state !== "read") throw new Error("the killed fixture is read");
  const job = {
    ...fixture.job,
    handle: "1-retire-guide-8-and-add-guide-validation-ru",
    title: "Retire guide 8 and add guide validation rule",
    branch: "armada/1-retire-guide-8-and-add-guide-validation-ru",
    workflow_id: "refactor",
    origin: "auto_detected",
    model: "sonnet",
    current_step_id: "plan",
    tasks: { done: 0, working: 0, open: 5, dropped: 0 },
  };
  return {
    ...fixture,
    job,
    watched: watchedRead({
      ...fixture.watched.detail,
      job,
      steps: [
        planRefused(),
        { ...freshStep("implement", "Restructure", 1), advance_gate: "auto_if_judge_passes" },
        { ...freshStep("handoff", "Review the change", 2), advance_gate: "human_always" },
      ],
      acceptance_criteria: [
        { criterion_id: "c1", text: "Guide 8 is removed from the catalogue", source: "judge" },
        { criterion_id: "c2", text: "A validation rule prevents guides without drawn pieces", source: "judge" },
      ],
      judge_question: {
        step_id: "plan",
        criterion_id: "structure_only",
        question: "Does this plan describe a change to structure only, with no intended change to behaviour?",
        expected: EXPECTED,
        produced: PRODUCED,
        consequence: CONSEQUENCE,
        asked_at: ASKED_AT,
      },
      stuck: { recourse: ["redispatch_job"], worktree_on_disk: false, drone_unheard: false, refused: [], refusals: 0 },
    }),
  };
}

test("a Job killed with a refusal waiting leads with its stop, and offers no Answer it", async () => {
  mount(onJob(killedWhileRefused()));
  // The Job's own read, landed: the Brief is the requester's words, which the
  // Board's row does not carry. Before it the lead is the row's to say.
  await expect.element(page.getByText(/Guide 8 is removed from the catalogue|selectors cannot be tested/).first()).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "This Job stopped" })).toBeVisible();
  const lead = document.querySelector(".armada-lead");
  expect(lead?.textContent).not.toMatch(/Judge refused/);
  expect(page.getByRole("button", { name: "Answer it" }).query()).toBeNull();
});
