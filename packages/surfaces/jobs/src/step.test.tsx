// The question a Drone asked, on the Job `#633` was filed from — a tests step
// that stopped `gate_undecided`: every Check passed and the Judge timed out.
//
// **A browser test, not a plain one**, because `questionOf` returns JSX, and
// what a reader sees is the markdown it draws rather than the elements it holds.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import type { JobDetail as JobWhole, JobSummary, StepDetail, Stuck } from "@armada/protocol";

import { mount, unmount } from "@armada/screens/src/mounted";
import { questionOf } from "./step";

afterEach(unmount);

const JOB_ID = "01M22TYSAE0023MADDP5ZQEYGW";

function job(): JobSummary {
  return {
    id: JOB_ID,
    handle: "2-refuse-a-merge-press-whose-chosen-comments",
    title: "Refuse a merge press whose chosen comments",
    status: "escalated",
    reason: { named: "gate_undecided" },
    workflow_id: "feature",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-11T09:00:00Z",
    current_step_id: "tests",
  };
}

/** Every Check passed, so the step's own record names no failed Check. */
function step(): StepDetail {
  return {
    step_id: "tests",
    label: "Tests",
    ordinal: 2,
    state: "stopped",
    checks: [{ kind: "manifest_check", name: "build", run: "cargo build" }],
    check_runs: [{ attempt: 1, name: "build", outcome: "passed" }],
    judge_checks: [{ criteria: 1, gaming_check: false }],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [
      { attempt: 1, outcome: "stopped", why: "gate_undecided", started_at: "2026-09-11T09:30:00Z" },
    ],
    verdicts: [{ attempt: 1, named: "failed", trigger: "gate_undecided" }],
    entered_at: "2026-09-11T09:30:00Z",
    updated_at: "2026-09-11T09:41:00Z",
  };
}

/** Fleet's own sentence, as it crosses the wire — lower-case, unpunctuated. */
const UNDECIDED_RAW = "the Judge did not answer inside its budget";

function stuck(showing: StepDetail, over: Partial<Stuck> = {}): Stuck {
  return {
    stopped_by: "gate_undecided",
    undecided: UNDECIDED_RAW,
    step_id: showing.step_id,
    recourse: ["rerun_gate"],
    worktree_on_disk: true,
    drone_unheard: false,
    refused: [],
    refusals: 0,
    ...over,
  };
}

function whole(showing: StepDetail, over: Partial<Stuck> = {}): JobWhole {
  return {
    job: job(),
    created_at: "2026-09-11T09:00:00Z",
    steps: [showing],
    acceptance_criteria: [],
    dependencies: [],
    stuck: stuck(showing, over),
  };
}

// The question a Drone asked, in the slot under Overview's lead. What each
// answer commits to is the Drone's own writing, so it draws as markdown.
test("a drone's answers draw their markdown", async () => {
  const showing = step();
  const asked: JobWhole = {
    ...whole(showing),
    asking: {
      question_id: "q1",
      step_id: showing.step_id,
      asked_at: "2026-09-11T09:40:00Z",
      question: "Should the column be its own job?",
      options: [
        { label: "Its own job", consequence: "Dispatch a migration first. **Nothing else starts** until it lands." },
        { label: "Fold it in", consequence: "The first job that needs `pending_at` adds it." },
      ],
    },
  };
  mount(<>{questionOf(asked, JOB_ID, false, false, () => {})}</>);
  await expect.element(page.getByText("Nothing else starts")).toBeVisible();
  expect(page.getByText("Nothing else starts").element().tagName).toBe("STRONG");
  expect(page.getByText("pending_at").element().tagName).toBe("CODE");
});
