// Five moments from the press to the gate: two requests dispatched at once, the
// wait on each of them, the answer on screen, and the Job frozen at the press.
//
// **A dispatched request is a Job from the press** — `job-statuses.toml`,
// `proposing` — so the first three moments are rows on the Board carrying the
// request as their title, and not a form waiting inside the composer.
//
// **The wire status is `awaiting_approval` and the word is `classifying`.**
// No registry declares a `classifying` status — `words.ts` carries the word as
// a draft and `job-statuses.toml` is what the badge reads, so the fixture
// carries the status a Fleet would send and the draft carries the word.
//
// **What locks, locks at approval** (#1530, 21 Sep): the gates and the tier
// map are editable up to the press and frozen after it. `approvedFrozen` is
// the moment after, and it is the one that says the issue has moved since.

import type { ProposalInFlight, ProposalSettled } from "@armada/protocol";
import type { GateView, LedgerRow, ProposalView } from "../../draft";
import type { ArcMoment } from "./arc-base";
import {
  ARC_APPROVED_AT,
  ARC_HANDLE,
  ARC_JOB_ID,
  ARC_NOW,
  arcCriterionViews,
  arcDetail,
  arcJob,
  arcManifests,
  arcResources,
  arcSteps,
  arcWatched,
  bugWorkflow,
  featureWorkflow,
} from "./arc-base";
import { ARC_LANDING, PROMPT, arcProposal, dispatchTyping } from "./arc-dispatch";
import { PROPOSER_BUDGET_MS, dispatchedFixture } from "./proposing";
import type { JobFixture } from "../fixture";

/**
 * How long each of the two calls has been out.
 *
 * **One fresh and one nearly out of time**, which is the whole difference
 * between the two waits: the fresh one has not reached the vendor yet, which is
 * the reach worth telling apart, and the other is past `PROPOSAL_IS_SLOW` and at
 * the register where the wait asks whether to keep waiting.
 */
const JUST_SENT_MS = 8_000;
const READING_FOR_MS = 460_000;

/** The second request, dispatched in the same sitting as the arc's own. */
// In markdown, as a person types one into Dispatch: the walk
// `markdownFromTheProposer` looks at it drawn as the Brief.
const SECOND_REQUEST =
  "The Cleared tab keeps rows whose worktree is gone and says nothing about the branch. " +
  "Say which of the two was given back on each row:\n\n" +
  "- the **worktree** alone\n" +
  "- the `branch` as well";
const SECOND_ID = "01M2D3ZF41002REQUEST0002";
const SECOND_HANDLE = "20-say-what-a-clear-gave-back";

/**
 * Both dispatched requests, as rows on the Board.
 *
 * **Two, because proposing several at once is the point.** The owner's words on
 * 30 Sep 2026: *"Something where I can propose multiple things at once and they
 * go off and get proposed. I dont need to sit on the screen and watch it."* One
 * of them is the arc's own Job, which becomes #1162 the moment the proposer
 * answers; until then its title is the request exactly as it was typed.
 */
function dispatched(): JobFixture[] {
  return [
    dispatchedFixture(
      {
        id: ARC_JOB_ID,
        handle: ARC_HANDLE,
        request: PROMPT,
        created_at: new Date(ARC_NOW - JUST_SENT_MS).toISOString(),
        says: "proposing — dispatched a moment ago, and the call has not reached the model",
      },
      ARC_NOW,
    ),
    dispatchedFixture(
      {
        id: SECOND_ID,
        handle: SECOND_HANDLE,
        request: SECOND_REQUEST,
        created_at: new Date(ARC_NOW - READING_FOR_MS).toISOString(),
        says: "proposing — most of the way through the proposer's budget",
      },
      ARC_NOW,
    ),
  ];
}

/**
 * What Fleet says about one call in flight.
 *
 * **Against the real clock, not the arc's**: the wait is drawn from the window's
 * own `now`, so a fixed instant here would read as a call that went out months
 * ago. `working-a-plan.ts` moves its instants for the same reason.
 */
function callOut(
  proposalId: string,
  outForMs: number,
  reached: ProposalInFlight["reached"],
  thinkingTokens?: number,
  settled?: ProposalSettled,
): ProposalInFlight {
  return {
    proposal_id: proposalId,
    client_ref: "bridge-1",
    model: "sonnet",
    since: new Date(Date.now() - outForMs).toISOString(),
    budget_ms: PROPOSER_BUDGET_MS,
    reached,
    ...(thinkingTokens === undefined ? {} : { thinking_tokens: thinkingTokens }),
    ...(settled === undefined ? {} : { settled }),
  };
}

/**
 * The four fields, each as the moment it has just landed.
 *
 * **The owner's order, and it is his reasoning rather than a layout** (30 Sep
 * 2026): the workflow decides the Job's shape, the title is what makes the row
 * recognisable, done-when is the goal, and the settings are the part he can
 * still change. Each entry here holds everything the one before it held, because
 * that is what a person watching one answer arrive sees — nothing is taken away
 * as the next field lands.
 */
const FILLING: readonly { name: string; says: string; settled: ProposalSettled }[] = [
  {
    name: "proposingWorkflowLanded",
    says: "Proposing — the workflow has landed and the row's title is still the request",
    settled: { workflow_id: "feature" },
  },
  {
    name: "proposingTitleLanded",
    says: "Proposing — the title has landed, and the row changed under the reader",
    settled: { workflow_id: "feature", title: "Say which of the two a clear gave back" },
  },
  {
    name: "proposingDoneWhenLanded",
    says: "Proposing — two done-when lines in, one line at a time",
    settled: {
      workflow_id: "feature",
      title: "Say which of the two a clear gave back",
      done_when: [
        "The Cleared tab names the branch on every row whose worktree is gone",
        "A row whose `branch` was also given back says so, and does not say it **twice**",
      ],
    },
  },
  {
    name: "proposingSettingsLanded",
    says: "Proposing — the settings have landed, model and all, and the answer is about to",
    settled: {
      workflow_id: "feature",
      title: "Say which of the two a clear gave back",
      done_when: [
        "The Cleared tab names the branch on every row whose worktree is gone",
        "A row whose `branch` was also given back says so, and does not say it **twice**",
      ],
      // **Both on one line of the answer, so both settle together** — the
      // settings are one field. The model is picked from what this machine
      // holds, which is the guard the owner took with it on 30 Sep 2026.
      settings: { urgency: "normal", model: "opus" },
    },
  },
  {
    name: "proposingModelLeftToConfiguration",
    says: "Proposing — the settings named no model, so configuration decides",
    settled: {
      workflow_id: "feature",
      title: "Say which of the two a clear gave back",
      done_when: [
        "The Cleared tab names the branch on every row whose worktree is gone",
        "A row whose `branch` was also given back says so, and does not say it **twice**",
      ],
      // **Absent stays absent**, which is the moment beside the one above: a
      // call that declines to name a model reaches configuration's choice and
      // never a default the proposer picked. Nothing stands in for it.
      settings: { urgency: "incident" },
    },
  },
];

/**
 * The second request, one field further on at each step. **The fixture the
 * owner watches fill**: four moments, one per field, each the same Job with one
 * more field settled than the moment before it.
 *
 * It is the second request rather than the arc's own, because that one is the
 * Job every later moment is about and its title is `ARC_TITLE` from
 * `proposingReview` onwards — a title landing here would be the arc's answer
 * arriving two moments early.
 */
export function proposingFilling(): ArcMoment[] {
  return FILLING.map((step) => ({
    name: step.name,
    says: step.says,
    fixtures: [
      dispatchedFixture(
        {
          id: ARC_JOB_ID,
          handle: ARC_HANDLE,
          request: PROMPT,
          created_at: new Date(ARC_NOW - JUST_SENT_MS).toISOString(),
          says: "proposing — dispatched a moment ago, and the call has not reached the model",
        },
        ARC_NOW,
      ),
      dispatchedFixture(
        {
          id: SECOND_ID,
          handle: SECOND_HANDLE,
          request: SECOND_REQUEST,
          created_at: new Date(ARC_NOW - READING_FOR_MS).toISOString(),
          says: step.says,
          settled: step.settled,
        },
        ARC_NOW,
      ),
      ...dispatchTyping().fixtures,
    ],
    proposing: callOut(
      "01M2D3ZF41002PROPOSAL002",
      READING_FOR_MS,
      "answering",
      1_840,
      step.settled,
    ),
    draft: {},
  }));
}

/**
 * What each step is gated by, as the proposer proposed it.
 *
 * `handoff` is the fourth state: the repository decides (#1530, 22 Sep). The
 * other three are what `feature.json` declares — a Judge on every step, and
 * Checks wherever the step writes.
 */
function gates(overridden: boolean): GateView[] {
  const handoff: GateView = {
    step_id: "handoff",
    checks: false,
    judge: false,
    you: overridden,
    repository_decides: "review_gate",
    overridden,
  };
  return [
    { step_id: "plan", checks: false, judge: true, you: false },
    { step_id: "implement", checks: true, judge: true, you: false },
    { step_id: "tests", checks: true, judge: true, you: false },
    handoff,
  ];
}

/** The proposal on screen, with a workflow chosen and gates to read. */
function classified(over: Partial<ProposalView> = {}): ProposalView {
  return arcProposal({ status: "awaiting_approval", gates: gates(false), ...over });
}

/**
 * The same proposal one press later, which every moment after it reads.
 *
 * **One spelling for the whole arc past the press.** What locks, locks at
 * approval (#1530, 21 Sep) — so a Job with a plan recorded or a Drone out is
 * held to exactly these gates, this tier map and this cap, and `approved_at` is
 * what draws them as a reading rather than as controls somebody could move
 * under a running Drone.
 */
export function arcApproved(): ProposalView {
  return classified({
    status: "approved",
    gates: gates(true),
    approved_at: ARC_APPROVED_AT,
  });
}

/** The Job itself, at the gate — no branch, no Drone, no step entered. */
function atTheGate(status: string, over = {}): JobFixture {
  const job = arcJob(status, { current_step_id: "plan", branch: undefined, ...over });
  const whole = arcDetail(job, arcSteps(), { branch: undefined });
  return {
    name: `${status} — the proposal is on screen and nothing has run`,
    job,
    watched: arcWatched(whole),
    // Both, because the proposal's picker offers every workflow this
    // repository declares and a picker with one option is not a picker.
    workflows: [featureWorkflow(), bugWorkflow()],
    manifests: arcManifests(),
    observed: { state: "none" },
    journalled: {
      state: "watching",
      jobId: ARC_JOB_ID,
      log: {
        skipped: 0,
        notes: [
          {
            at: "2026-09-22T09:08:00Z",
            by: "fleet",
            level: "info",
            seq: 1,
            msg: "The proposer read the linked issue and chose the feature workflow.",
          },
        ],
      },
    },
    resources: { state: "read", jobId: ARC_JOB_ID, resources: arcResources("none") },
    recorded: {
      footprint: { state: "none" },
      handed: { state: "none" },
      evidence: { state: "none" },
      diff: { state: "none" },
      remarks: { state: "none" },
    },
    checkOutputs: {},
    frames: {},
    now: ARC_NOW,
  };
}

/** What the Record holds before anything ran: one press, and what it froze. */
function frozenRecord(): LedgerRow[] {
  return [
    {
      at: "2026-09-22T09:06:00Z",
      coord: null,
      actor: "fleet",
      kind: "proposed",
      what: "the proposer read armada/1162 and chose the feature workflow",
      outcome: "four steps, a Judge on each",
      cursor: 1,
    },
    {
      at: ARC_APPROVED_AT,
      coord: null,
      actor: "person",
      kind: "status_queued",
      what: "approved the dispatch",
      outcome: "the workflow, the gates and the tier map are frozen",
      cursor: 2,
    },
  ];
}

/**
 * The press, and nothing open. **Where Dispatch leaves you**: two rows on the
 * Board, each carrying its request, and nobody sitting on a screen.
 */
export function proposingDispatched(): ArcMoment {
  return {
    name: "proposingDispatched",
    says: "Proposing — two requests dispatched at once, and neither has been read",
    fixtures: [...dispatched(), ...dispatchTyping().fixtures],
    draft: {},
  };
}

/** The arc's own request, opened seconds after the press. */
export function proposingReading(): ArcMoment {
  return {
    name: "proposingReading",
    says: "Proposing — the request opened, and the call has not reached the model yet",
    fixtures: [...dispatched(), ...dispatchTyping().fixtures],
    opens: ARC_JOB_ID,
    proposing: callOut("01M2D3ZF41001PROPOSAL001", JUST_SENT_MS, "starting"),
    draft: {},
  };
}

/** The other request, most of the way through the budget and worth stopping. */
export function proposingSlow(): ArcMoment {
  return {
    name: "proposingSlow",
    says: "Proposing — the second request is nearly out of budget, and the wait asks",
    fixtures: [...dispatched(), ...dispatchTyping().fixtures],
    opens: SECOND_ID,
    proposing: callOut("01M2D3ZF41002PROPOSAL002", READING_FOR_MS, "thinking", 1_840),
    draft: {},
  };
}

export function proposingReview(): ArcMoment {
  const before = dispatchTyping();
  return {
    name: "proposingReview",
    says: "Classifying — a workflow, four gate rows and a tier map, all still editable",
    fixtures: [atTheGate("awaiting_approval"), ...before.fixtures],
    opens: ARC_JOB_ID,
    draft: {
      prompt: before.draft.prompt,
      proposal: classified(),
      landing: ARC_LANDING,
      criteria: arcCriterionViews(),
    },
  };
}

export function approvedFrozen(): ArcMoment {
  const job = atTheGate("queued", { started_at: ARC_APPROVED_AT, queued_reason: "waiting_on_resources" });
  return {
    name: "approvedFrozen",
    says: "Approved — frozen at the press, and the issue has been edited since",
    fixtures: [{ ...job, name: "queued — approved, and the linked issue has moved since" }],
    opens: ARC_JOB_ID,
    draft: {
      proposal: arcApproved(),
      landing: ARC_LANDING,
      // The Job keeps the words it froze, and says the issue has moved since
      // (#1530, 22 Sep). The instant is the issue's edit, never the freeze.
      criteria: arcCriterionViews("2026-09-22T10:02:00Z"),
      record: frozenRecord(),
    },
  };
}
