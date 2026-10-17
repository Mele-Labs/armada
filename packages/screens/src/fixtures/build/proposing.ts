// A request that has been dispatched and not yet read — a Job with no workflow,
// no steps, no plan and no Drone, whose title is the request as it was typed.
//
// **The one fixture that is honestly this empty.** `job-statuses.toml` says
// `proposing` is the only status with no frozen workflow at all, so there is
// nothing for the run to draw, nothing for the plan to draw and nothing on the
// machine — which is what every reader here has to hold without looking broken.
//
// **Nothing on the wire says how long the call has been out.** That is
// `ProposalInFlight`, which hangs off no Job — so a moment carrying one
// publishes it as `BridgeState.proposing` and this file carries none.

import type { JobDetail, JobSummary, ProposalSettled, Watched } from "@armada/protocol";
import type { JobFixture } from "../fixture";
import { filled } from "../../proposal";
import { CREATED_AT, JOB_HANDLE, JOB_ID, MANIFEST_ID, manifest, NOW } from "./base";

/**
 * Fleet's own ceiling for one proposer call, ten provisional minutes —
 * `PROVISIONAL_PROPOSER_BUDGET`. What a wait is drawn against, so a moment
 * saying how long the call has been out says how much of it is left.
 */
export const PROPOSER_BUDGET_MS = 600_000;

/** One dispatched request, before the proposer has answered. */
export type Dispatched = {
  id: string;
  handle: string;
  /** The request as it was typed. **The Job's title, and nobody wrote a title.** */
  request: string;
  /** When Dispatch was pressed, which is when the Job was created. */
  created_at: string;
  /** The state, as a sentence — the picker lists it. */
  says: string;
  /** Which model is reading it. */
  model?: string;
  /**
   * What the proposer has settled so far, where it has settled anything.
   *
   * **Folded onto the row and the detail, through the one function that folds
   * it** — `filled`. A settled field is the Job becoming more complete, so a
   * fixture carrying one is a fixture whose Job has that field, and every reader
   * on the Board and on the page draws it with no arm for a proposal.
   */
  settled?: ProposalSettled;
};

/**
 * The row. **`workflow_id` is empty, and that is the state rather than a gap**
 * — the proposer has not chosen one, and an id here would name a workflow
 * nothing froze. No `current_step_id`, no `branch`, no `started_at`,
 * no `assigned_drone` and no `tasks`: none of the five exists yet.
 */
export function dispatchedRow(one: Dispatched): JobSummary {
  return {
    id: one.id,
    handle: one.handle,
    title: one.request,
    status: "proposing",
    workflow_id: "",
    owner_manifest_id: MANIFEST_ID,
    origin: "manual",
    urgency: "normal",
    atomic: false,
    model: one.model ?? "sonnet",
    created_at: one.created_at,
  };
}

/**
 * The whole Job, as `GET /jobs/:job_id` would answer it: the row, no steps, no
 * criteria and no facts. **No `spend` either** — the proposer's own call is
 * Fleet's and no Drone has cost this Job anything.
 */
function dispatchedDetail(row: JobSummary, created: string): JobDetail {
  return {
    job: row,
    created_at: created,
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    // **The starting value, because nobody has changed anything.** A Job this
    // early carries no settings at all, and any other value here would make
    // Overview's Settings card and the strip's own figure say a change was made.
    when_blocked: "ask_me",
  };
}

/**
 * The fixture. **`resources` is `none`, not an empty reading**: there is no
 * worktree and no process to look at, so the Pulse card says nothing was read
 * rather than reading a machine this Job is not on.
 */
export function dispatchedFixture(one: Dispatched, now: number): JobFixture {
  const settling = filled(dispatchedRow(one), dispatchedDetail(dispatchedRow(one), one.created_at), one.settled);
  const row = settling.job;
  const watched: Watched = {
    state: "read",
    jobId: one.id,
    detail: settling.detail,
  };
  return {
    name: one.says,
    job: row,
    watched,
    // No workflow is **frozen**, so this Job's own roster is empty — a settled
    // workflow is a choice and not a freeze, and the freeze is what
    // `proposing -> awaiting_approval` does. The Board still holds whatever its
    // other Jobs declare.
    workflows: [],
    manifests: [manifest()],
    observed: { state: "none" },
    journalled: { state: "none" },
    resources: { state: "none" },
    recorded: {
      footprint: { state: "none" },
      handed: { state: "none" },
      evidence: { state: "none" },
      diff: { state: "none" },
      remarks: { state: "none" },
    },
    checkOutputs: {},
    frames: {},
    now,
  };
}

/**
 * The roster's own: the Bug Job's id and handle at the moment before any of it
 * existed, which is what `every-state` lists and what `asRow` renames.
 *
 * **The title is the request**, so the row reads as words somebody typed rather
 * than as a title the proposer has not written yet.
 */
export function proposing(): JobFixture {
  return dispatchedFixture(
    {
      id: JOB_ID,
      handle: JOB_HANDLE,
      request:
        "Split the settings reducer so the selectors can be tested without building the whole store.",
      created_at: CREATED_AT,
      says: "proposing — dispatched, and the proposer has not answered yet",
    },
    NOW,
  );
}
