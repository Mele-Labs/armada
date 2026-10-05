// Three moments that are several Jobs at once: work landing in order, twice,
// and a wave of Jobs under one plan.
//
// **No kind name anywhere** (#1530, 22 Sep). A Job is a Job: the parent here
// holds a plan and one approval, its members are Jobs of their own, and a
// screen built on this says "three pull requests, landing in order" rather
// than naming a shape.
//
// **`dispatched_by` is what makes a member a member** — the wire field
// `jobMembersOf` reads — and how one member's work reaches the next is the
// draft's `MemberLink`, which nothing on the wire says at all.

import type { CommandInFlight, Criterion, JudgeQuestion, StepDetail } from "@armada/protocol";

import type { JobMembersView, LandingRule, MemberView } from "../../draft";
import type { Outstanding } from "../../outstanding";
import type { JobFixture } from "../fixture";
import type { ArcMoment } from "./arc-base";
import { ARC_NOW, arcStep, featureWorkflow } from "./arc-base";
import { lightFixture } from "./light";
import type { LightJob } from "./light";
import { epicWorkflow } from "./kinds-workflows";

const PARENT_ID = "01M2D7A1XK001SETTINGSPLIT";
const PARENT_TITLE = "Move the settings store out in three landings";

const MEMBER_IDS = {
  a: "01M2D7A1XK001MEMBER00000A",
  b: "01M2D7A1XK001MEMBER00000B",
  c: "01M2D7A1XK001MEMBER00000C",
};

/** One member Job, dispatched by the parent above. */
function member(over: Partial<LightJob> & Pick<LightJob, "id" | "handle" | "title" | "status" | "at" | "says" | "created_at">): JobFixture {
  return lightFixture(
    {
      workflow: featureWorkflow(),
      steps: [
        arcStep("plan", "Plan the change", 1),
        arcStep("implement", "Implement", 2),
        arcStep("tests", "Write tests", 3),
        arcStep("handoff", "Review the change", 4),
      ],
      branch: `armada/${over.handle}`,
      ...over,
      row: { origin: "sub_dispatched", dispatched_by: PARENT_ID, ...over.row },
    },
    ARC_NOW,
  );
}

/** The parent: it holds the plan and the one approval, and lands nothing itself. */
function parent(): JobFixture {
  return lightFixture(
    {
      id: PARENT_ID,
      handle: "21-move-the-settings-store-out",
      title: PARENT_TITLE,
      status: "running",
      workflow: featureWorkflow(),
      at: "implement",
      steps: [
        arcStep("plan", "Plan the change", 1),
        arcStep("implement", "Implement", 2),
        arcStep("tests", "Write tests", 3),
        arcStep("handoff", "Review the change", 4),
      ],
      says: "running — the parent holds the plan, and its members do the landing",
      created_at: "2026-09-22T08:00:00Z",
      started_at: "2026-09-22T08:02:00Z",
      detail: { write_targets: ["packages/settings/src/"] },
    },
    ARC_NOW,
  );
}

/** The three members, in the order they land. */
function members(): JobFixture[] {
  return [
    member({
      id: MEMBER_IDS.a,
      handle: "22-give-the-store-one-shape",
      title: "Give the store one shape",
      status: "completed_success",
      at: "handoff",
      says: "completed_success — the first landing merged",
      created_at: "2026-09-22T08:05:00Z",
      started_at: "2026-09-22T08:06:00Z",
      ended_at: "2026-09-22T09:40:00Z",
      row: { landed: "merged", tasks: { done: 4, working: 0, open: 0, dropped: 0 } },
      detail: {
        write_targets: ["packages/settings/src/store.ts"],
        delivery: {
          commit: "a19c4b7",
          pushed: "origin/armada/22-give-the-store-one-shape",
          pull_request: "https://git.example/armada/pull/1591",
          landed: "merged",
        },
      },
    }),
    member({
      id: MEMBER_IDS.b,
      handle: "23-read-the-store-through-selectors",
      title: "Read the store through selectors",
      status: "awaiting_review",
      at: "handoff",
      says: "awaiting_review — the second landing is waiting on you",
      created_at: "2026-09-22T08:05:00Z",
      started_at: "2026-09-22T09:41:00Z",
      row: { asking: true, tasks: { done: 3, working: 0, open: 0, dropped: 0 } },
      detail: {
        write_targets: ["packages/settings/src/read.ts", "apps/desktop/src/renderer/"],
        judge_question: MEMBER_QUESTION,
        delivery: {
          commit: "c72d0e9",
          pushed: "origin/armada/23-read-the-store-through-selectors",
          pull_request: "https://git.example/armada/pull/1598",
        },
      },
    }),
    member({
      id: MEMBER_IDS.c,
      handle: "24-drop-the-store-singleton",
      title: "Drop the store singleton",
      status: "running",
      at: "implement",
      says: "running — the third landing is working on top of the second",
      created_at: "2026-09-22T08:05:00Z",
      started_at: "2026-09-22T10:20:00Z",
      row: { tasks: { done: 1, working: 1, open: 3, dropped: 0 } },
      detail: { write_targets: ["packages/settings/src/", "crates/config/src/"] },
    }),
  ];
}

/**
 * The refusal the second member is holding, answered from the parent.
 *
 * **The wire's own `JudgeQuestion`**, against the member's Job id — answering
 * it here sends the same `answer_judge` as answering it on that Job, which is
 * the whole reason a person never has to leave this screen to clear it.
 */
const MEMBER_QUESTION = {
  step_id: "handoff",
  criterion_id: "02-selectors-cover-the-empty-store",
  question: "Does every selector answer for a store nothing has written to yet?",
  expected: "A case for the empty store beside each selector",
  produced: "Two of the six selectors are exercised only against a filled store",
  consequence: "A first launch reads undefined through those two, before anything is saved",
  asked_at: "2026-09-22T10:48:00Z",
};

/**
 * What the parent's members are, and how each one's work reaches the one
 * before it.
 *
 * **The first carries no link**, because nothing is before it; its pull
 * request targets where the Job lands and `landed` says that pull request
 * merged. The other two carry a link each, so both moments together draw all
 * three of `docs/concepts/landing.md`'s edges.
 */
function membersView(third: MemberView["link"]): JobMembersView {
  return {
    job: PARENT_ID,
    title: PARENT_TITLE,
    members: [
      {
        job: MEMBER_IDS.a,
        title: "Give the store one shape",
        status: "completed_success",
        landed: true,
        landed_at: "2026-09-22T09:40:00Z",
        branch: "armada/22-give-the-store-one-shape",
        pull_request: "https://git.example/armada/pull/1591",
        scope: ["packages/settings/src/store.ts"],
        tasks: { done: 4, working: 0, open: 0, dropped: 0 },
      },
      {
        // It waits on the snapshot the first member's merge publishes, not on
        // the merge itself — the `published` edge, and the only one of the
        // three that needs nothing at the parent's level.
        job: MEMBER_IDS.b,
        title: "Read the store through selectors",
        status: "awaiting_review",
        link: "published",
        landed: false,
        branch: "armada/23-read-the-store-through-selectors",
        pull_request: "https://git.example/armada/pull/1598",
        scope: ["packages/settings/src/read.ts", "apps/desktop/src/renderer/"],
        tasks: { done: 3, working: 0, open: 0, dropped: 0 },
        question: MEMBER_QUESTION,
      },
      {
        job: MEMBER_IDS.c,
        title: "Drop the store singleton",
        status: "running",
        link: third,
        landed: false,
        branch: "armada/24-drop-the-store-singleton",
        scope: ["packages/settings/src/", "crates/config/src/"],
        tasks: { done: 1, working: 1, open: 3, dropped: 0 },
      },
    ],
  };
}

/** One pull request per member, and the parent is done when all of them land. */
function landingInOrder(prMode: LandingRule["pr_mode"]): LandingRule {
  return {
    target: "main",
    from_ref: "main",
    prs: "group",
    branching: "group",
    pr_mode: prMode,
    complete_when: "all_members_landed",
    land_together: [],
  };
}

export function membersStacked(): ArcMoment {
  return {
    name: "stacked",
    says: "Three pull requests landing in order — the third is stacked on the second",
    fixtures: [parent(), ...members()],
    opens: PARENT_ID,
    draft: { members: membersView("stacked"), landing: landingInOrder("ready") },
  };
}

export function membersMerged(): ArcMoment {
  return {
    name: "merged",
    // Parked: the third member's work is merged into the one before it and its
    // own pull request is a draft, so nothing is asked of a reviewer yet.
    says: "Three pull requests landing in order — the third is merged into the second and parked",
    fixtures: [parent(), ...members()],
    opens: PARENT_ID,
    draft: { members: membersView("merged"), landing: landingInOrder("draft") },
  };
}

export const WAVE_ID = "01M2D8B2YL001ERRORCONTRACT";

/**
 * The step that recorded the split, with the Judge `epic.json` puts on it —
 * two criteria, both met on the pass being run now.
 */
function planStep(): StepDetail {
  const step = arcStep("plan", "Plan the wave", 1);
  return {
    ...step,
    // `epic.json` lets it create the wave's Jobs.
    may_dispatch_jobs: true,
    state: "advanced",
    // `epic.json` declares it, and it is what says which step recorded the
    // split — `tab-plan.tsx` reads the check, never the workflow's name.
    checks: [{ kind: "plan_recorded" }],
    judged: [
      { attempt: 2, criterion_id: "draws_the_split", verdict: "met" },
      { attempt: 2, criterion_id: "each_piece_carries_its_own_brief", verdict: "met" },
    ],
    attempts: [{ attempt: 2, outcome: "advanced", started_at: "2026-09-22T07:12:00Z", ended_at: "2026-09-22T07:19:00Z" }],
    verdicts: [{ attempt: 2, named: "passed" }],
  };
}

/**
 * The step the loop returns through. `pass` and `verdict_routing_target` are
 * what draw the iteration count, and both are on the wire — `epic.json` caps
 * the loop at five.
 */
function rollUpStep(): StepDetail {
  return {
    ...arcStep("roll_up", "Roll up the wave", 2),
    phase: "delivery",
    pass: { number: 2, of: 5 },
    verdict_routing_target: "plan",
  };
}

/**
 * The wave's parent, on the two steps `epic.json` declares. **Queued, standing
 * after its plan**: Approve the plan released the wave and the parent waits on
 * it, holding no Drone, as Fleet holds it since 23.11.
 */
export function waveParent(): JobFixture {
  return lightFixture(
    {
      id: WAVE_ID,
      handle: "31-carry-the-error-contract-everywhere",
      title: "Carry the error contract through every surface",
      status: "queued",
      workflow: epicWorkflow(),
      at: "plan",
      steps: [planStep(), rollUpStep()],
      says: "queued — waiting on the second pass of its wave, three Jobs waiting on you",
      created_at: "2026-09-22T05:10:00Z",
      started_at: "2026-09-22T05:12:00Z",
      detail: { wave_rounds: WAVE_ROUNDS },
    },
    ARC_NOW,
  );
}

/**
 * What each pass of the parent's plan recorded as its approach, as
 * `JobDetail.wave_rounds` serves it since 23.14 (#1692): the whole paragraph,
 * of which the strip reads the first sentence.
 */
export const WAVE_ROUNDS = [
  {
    pass: 1,
    approach:
      "The seam as one Job. Every refusal is read in one place before any surface draws it, so the surfaces have one shape to follow.",
  },
  {
    pass: 2,
    approach:
      "The seam first, then every surface that reads it. The roll-up sent the first pass back as too large to review, so the seam lands alone and each surface follows it.",
  },
];

const WAVE_IDS = {
  a: "01M2D8B2YL001WAVE00000A",
  b: "01M2D8B2YL001WAVE00000B",
  c: "01M2D8B2YL001WAVE00000C",
  d: "01M2D8B2YL001WAVE00000D",
  e: "01M2D8B2YL001WAVE00000E",
  // The first pass's two, which the roll-up sent back.
  f: "01M2D8B2YL001WAVE00000F",
  g: "01M2D8B2YL001WAVE00000G",
};

/**
 * The refusal the Judge opened on the Job at the gate, and the one that
 * stopped the Job holding a Drone. Both are `JobDetail.judge_question`, which
 * is what the existing card is drawn from.
 */
const GATE_REFUSAL: JudgeQuestion = {
  step_id: "handoff",
  criterion_id: "every_code_reaches_the_journal",
  question: "Does every refusal the seam produces reach the journal with its code?",
  expected: "A refusal with an unknown code is journalled as error.unknown, with the raw code beside it.",
  produced: "An unknown code is journalled with an empty code field and the raw value is dropped.",
  consequence: "A person reading the journal after an unknown refusal cannot tell which code arrived.",
  asked_at: "2026-09-22T10:48:00Z",
};

const BLOCKED_REFUSAL: JudgeQuestion = {
  step_id: "implement",
  criterion_id: "the_half_is_named",
  question: "Does the message say which half refused — Bridge or Fleet?",
  expected: "A transport failure names the side that refused, as error-contract.md requires.",
  produced:
    "The message reads `the request failed` and names **neither side**:\n\n" +
    "- not Bridge, which sent it\n" +
    "- not Fleet, which refused it",
  consequence: "A person cannot tell whether to restart Fleet or reopen the window.",
  asked_at: "2026-09-22T11:02:00Z",
};

/** The command the last Job's Drone stopped inside, which the Manifest has not cleared. */
const UNCLEARED: CommandInFlight = {
  call: "call_gh_api_1",
  step_id: "implement",
  asked_at: "2026-09-22T11:11:00Z",
  tool: "Bash",
  detail: "gh api repos/:owner/:repo/issues/1544/comments",
  truncated: false,
  length: 48,
  offers: ["allow_for_job", "always_allow", "reject"],
  rules: ["gh", "gh api"],
  suggested_rule: "gh api",
};

/**
 * One Job of the wave as the plan wrote it: which pass dispatched it, what it
 * waits on, and the brief and expectations `plan.md` handed its Drone. What it
 * has spent and how far through its tasks it is are the Board's, drawn here
 * because the wave's own view does not carry the row.
 */
type WaveChild = {
  id: string;
  handle: string;
  title: string;
  status: string;
  round: number;
  waits: string[];
  landed?: "merged" | "closed_unmerged";
  brief: string;
  expects: string[];
  cost_micros?: number;
  tasks?: { done: number; of: number };
};

/**
 * What a child expects, as `JobDetail.acceptance_criteria` carries it: one
 * criterion a line, minted `c1`, `c2` as Fleet mints them, and answered by the
 * Judge, which reads words it was handed.
 */
function criteriaOf(expects: readonly string[]): Criterion[] {
  return expects.map((text, at) => ({ criterion_id: `c${at + 1}`, text, source: "judge" }));
}

/**
 * What the two passes dispatched, and which waits on which.
 *
 * **The order is the work's own.** The seam refuses first; the two surfaces
 * that carry the code follow it; naming which half refused follows the toast
 * that would say so; and the second error shape can only be dropped once every
 * surface carries the first — so it waits on both, and never the reverse.
 *
 * **The first pass is history.** It tried the seam as one Job; the roll-up
 * sent it back as too large to review, and the second pass split it. Its Jobs
 * are still Jobs, so a person pressing Wave 1 reads what they did.
 *
 * **No landing link, because a wave is not a landing order.** One of these
 * waits on another because its work depends on that work, and each lands when
 * it is done — nothing says the fourth merges after the third.
 */
const WAVE_CHILDREN: WaveChild[] = [
  {
    id: WAVE_IDS.f,
    handle: "26-list-the-refusal-codes",
    title: "List every code Fleet refuses with",
    status: "completed_success",
    round: 1,
    waits: [],
    landed: "merged",
    brief:
      "Write down every refusal code Fleet can send, with one line on what each means, in one file Bridge can read.",
    expects: ["Every code Fleet sends is in the list", "Each code has a one-line meaning"],
    cost_micros: 940_000,
    tasks: { done: 2, of: 2 },
  },
  {
    id: WAVE_IDS.g,
    handle: "27-handle-refusals-at-the-seam",
    title: "Handle every refusal at the seam",
    status: "superseded",
    round: 1,
    waits: [WAVE_IDS.f],
    landed: "closed_unmerged",
    brief:
      "Catch every refusal where Bridge meets Fleet and turn it into the contract's shape, in one change.",
    expects: ["Nothing past the seam sees a raw refusal"],
    cost_micros: 3_420_000,
    tasks: { done: 6, of: 6 },
  },
  {
    id: WAVE_IDS.a,
    handle: "32-refuse-an-unknown-code",
    title: "Refuse an unknown code at the seam",
    status: "completed_success",
    round: 2,
    waits: [],
    landed: "merged",
    brief:
      "Make the seam turn any code it does not know into error.unknown, and keep the raw code beside it. Nothing past the seam should see a code it can't name.",
    expects: ["An unknown code arrives as error.unknown", "The raw code is kept, not dropped"],
    cost_micros: 2_140_000,
    tasks: { done: 4, of: 4 },
  },
  {
    id: WAVE_IDS.b,
    handle: "33-name-the-fault-in-the-toast",
    title: "Name the fault in the toast",
    status: "completed_success",
    round: 2,
    waits: [WAVE_IDS.a],
    landed: "merged",
    brief: "Show the refusal's code and what it means in the toast, instead of \"Something went wrong\".",
    expects: ["Every refusal toast names its code", "An unknown code reads as unknown, not blank"],
    cost_micros: 1_380_000,
    tasks: { done: 3, of: 3 },
  },
  {
    id: WAVE_IDS.c,
    handle: "34-carry-the-code-into-the-log",
    title: "Carry the code into the journal",
    status: "awaiting_review",
    round: 2,
    waits: [WAVE_IDS.a],
    brief:
      "Write every refusal to the journal with its code, so someone reading it later can tell what arrived.",
    expects: [
      "Each refusal's journal line carries its code",
      "An unknown code is journalled with the raw value beside it",
    ],
    cost_micros: 2_650_000,
    tasks: { done: 5, of: 5 },
  },
  {
    id: WAVE_IDS.d,
    handle: "35-say-which-half-refused",
    title: "Say which half refused",
    status: "escalated",
    round: 2,
    waits: [WAVE_IDS.b],
    brief:
      "When a request fails, say whether Bridge or Fleet refused it. The two need different fixes, so the message has to tell them apart.",
    expects: ["A transport failure names the side that refused", "The wording matches error-contract.md"],
    cost_micros: 1_910_000,
    tasks: { done: 2, of: 4 },
  },
  {
    id: WAVE_IDS.e,
    handle: "36-drop-the-second-error-shape",
    title: "Drop the second error shape",
    status: "running",
    round: 2,
    waits: [WAVE_IDS.c, WAVE_IDS.d],
    brief:
      "Remove the old message-only error shape now that every surface reads the contract's. Delete it rather than wrapping it.",
    expects: ["Nothing builds the old shape", "Every test that used it reads the new one"],
    tasks: { done: 1, of: 6 },
  },
];

/**
 * Each child as a Board fixture, dispatched by the wave's parent. **A child at
 * `awaiting_approval` has not run**: no step entered, no start, no branch.
 */
export function waveChildren(
  children: readonly WaveChild[] = WAVE_CHILDREN,
): { child: WaveChild; fixture: JobFixture }[] {
  return children.map((child, at) => {
    const { id, handle, title, status, round } = child;
    const done = status === "completed_success" || status === "superseded";
    const proposed = status === "awaiting_approval";
    const created = round === 1 ? "2026-09-22T05:20:00Z" : "2026-09-22T07:20:00Z";
    const started = round === 1 ? "2026-09-22T05:22:00Z" : "2026-09-22T07:22:00Z";
    const ended = round === 1 ? "2026-09-22T06:40:00Z" : "2026-09-22T09:05:00Z";
    return {
      child,
      fixture: lightFixture(
        {
          id,
          handle,
          title,
          status,
          workflow: featureWorkflow(),
          at: proposed ? "plan" : done || status === "awaiting_review" ? "handoff" : "implement",
          steps: [
            arcStep("plan", "Plan the change", 1),
            arcStep("implement", "Implement", 2),
            arcStep("tests", "Write tests", 3),
            arcStep("handoff", "Review the change", 4),
          ],
          says: `${status} — one Job of the wave`,
          created_at: created,
          started_at: proposed ? undefined : started,
          ended_at: done ? ended : undefined,
          branch: proposed ? undefined : `armada/${handle}`,
          row: {
            origin: "sub_dispatched",
            dispatched_by: WAVE_ID,
            // The pass that proposed it, which is what says it is a wave's
            // (23.11), and when its pull request merged, where it did.
            dispatched_pass: round,
            // What it waits on, on its own row since 23.14 (#1692).
            ...(child.waits.length === 0 ? {} : { waits_on: child.waits }),
            ...(child.landed === "merged" ? { merged_at: ended } : {}),
            ...(child.landed === undefined ? {} : { landed: child.landed }),
            // The flag that lifts a row into Needs you, and what tells a running
            // Job with a Drone inside a call apart from one simply working.
            ...(id === WAVE_IDS.e && !proposed ? { asking: true } : {}),
          },
          // A proposed child has asked nothing: it shares its id with the Job
          // it becomes, not with what that Job ran into.
          detail: proposed
            ? {}
            : {
                ...(id === WAVE_IDS.c ? { judge_question: GATE_REFUSAL } : {}),
                ...(id === WAVE_IDS.d ? { judge_question: BLOCKED_REFUSAL } : {}),
                ...(id === WAVE_IDS.e ? { command_waiting: UNCLEARED, when_blocked: "ask_me" } : {}),
              },
        },
        ARC_NOW + at,
      ),
    };
  });
}

/** The three questions the wave is holding open, as main gathers them. */
export function waveQuestions(): Outstanding[] {
  return [
    { kind: "judge", job_id: WAVE_IDS.c, question: GATE_REFUSAL },
    { kind: "judge", job_id: WAVE_IDS.d, question: BLOCKED_REFUSAL },
    { kind: "command", job_id: WAVE_IDS.e, waiting: UNCLEARED },
  ];
}

export function epicWave(): ArcMoment {
  const children = waveChildren();
  return {
    name: "wave",
    says: "A wave — the second pass of the split, with three Jobs waiting on you",
    fixtures: [waveParent(), ...children.map((one) => one.fixture)],
    opens: WAVE_ID,
    questions: waveQuestions(),
    draft: {
      wave: {
        job: WAVE_ID,
        title: "Carry the error contract through every surface",
        // Pass 1 tried the seam as one Job and was sent back by the roll-up;
        // pass 2 is the plan being run now. A loop return replaces `plan.md`
        // whole, so the first is history.
        rounds: [
          { round: 1, says: "the seam", live: false },
          { round: 2, says: "every surface", live: true },
        ],
        judged_by: "haiku",
        jobs: children.map(({ child }) => ({
          job: child.id,
          title: child.title,
          status: child.status,
          handle: child.handle,
          round: child.round,
          waits_on: child.waits,
          ...(child.landed === undefined ? {} : { landed: child.landed }),
          facts: child.brief,
          criteria: criteriaOf(child.expects),
          ...(child.cost_micros === undefined ? {} : { cost_micros: child.cost_micros }),
          ...(child.tasks === undefined ? {} : { tasks: child.tasks }),
        })),
      },
    },
  };
}

/**
 * The same parent back at its plan gate: the first wave rolled up, and the
 * second pass's split waiting on a person. **`evidence_type: "plan"` is what
 * Overview's gate switches on** to draw Plan's review rather than the work's.
 */
function waveParentAtItsGate(): JobFixture {
  const fixture = lightFixture(
    {
      id: WAVE_ID,
      handle: "31-carry-the-error-contract-everywhere",
      title: "Carry the error contract through every surface",
      status: "awaiting_review",
      workflow: epicWorkflow(),
      at: "plan",
      steps: [
        {
          ...planStep(),
          state: "awaiting_human",
          attempts: [{ attempt: 2, outcome: "awaiting_human", started_at: "2026-09-22T07:12:00Z" }],
          verdicts: [],
        },
        rollUpStep(),
      ],
      says: "awaiting_review — the second pass's split is waiting on you",
      created_at: "2026-09-22T05:10:00Z",
      started_at: "2026-09-22T05:12:00Z",
      detail: { wave_rounds: WAVE_ROUNDS },
    },
    ARC_NOW,
  );
  return {
    ...fixture,
    recorded: {
      ...fixture.recorded,
      evidence: {
        state: "read",
        jobId: WAVE_ID,
        steps: [
          {
            step_id: "plan",
            evidence_type: "plan",
            claimed: "The seam first, then every surface that reads a refusal, as five Jobs.",
            shown_by: ".armada/deliverables/31-carry-the-error-contract-everywhere/plan.2.md",
          },
        ],
      },
    },
  };
}

/**
 * The second pass's split as the gate holds it: the same five Jobs, each a
 * real Job at `awaiting_approval` dispatched by the Epic, carrying its brief,
 * what it expects and what it waits on — and nothing it has spent, landed or
 * done, because none of them has run.
 * Held by the decision that approving an Epic's plan releases its wave.
 */
function proposedWave(): WaveChild[] {
  return WAVE_CHILDREN.filter((child) => child.round === 2).map(
    ({ landed: _landed, cost_micros: _spent, tasks: _tasks, ...child }) => ({
      ...child,
      status: "awaiting_approval",
    }),
  );
}

/**
 * An Epic Job whose plan step waits on a person, after one wave: what
 * Overview's plan gate draws for a wave rather than a task board. **The split
 * being approved is drawn**: wave 2's Jobs exist at `awaiting_approval`, so
 * the Board lists them too, and Approve the plan releases every one (#1694).
 */
export function epicPlanReview(): ArcMoment {
  const firsts = waveChildren().filter(({ child }) => child.round === 1);
  const proposed = waveChildren(proposedWave());
  const all = [...firsts, ...proposed];
  return {
    name: "planReview",
    says: "A wave — the first rolled up, and the second pass's split waiting on your review",
    fixtures: [waveParentAtItsGate(), ...all.map((one) => one.fixture)],
    opens: WAVE_ID,
    draft: {
      wave: {
        job: WAVE_ID,
        title: "Carry the error contract through every surface",
        // The proposed split is the live one: it is what the gate approves.
        rounds: [
          { round: 1, says: "the seam", live: false },
          { round: 2, says: "every surface", live: true },
        ],
        judged_by: "haiku",
        jobs: all.map(({ child }) => ({
          job: child.id,
          title: child.title,
          status: child.status,
          handle: child.handle,
          round: child.round,
          waits_on: child.waits,
          ...(child.landed === undefined ? {} : { landed: child.landed }),
          facts: child.brief,
          criteria: criteriaOf(child.expects),
          ...(child.cost_micros === undefined ? {} : { cost_micros: child.cost_micros }),
          ...(child.tasks === undefined ? {} : { tasks: child.tasks }),
        })),
      },
    },
  };
}
