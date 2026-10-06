// A Job while somebody is still deciding what it will be. Draft, for
// `crates/ipc/src/detail.rs` and `crates/ipc/src/limits.rs`.
//
// Source of truth today: `JobDetail` at `awaiting_approval`, `StepDetail`'s
// `advance_gate`, `checks`, `judge_checks` and `overridden`, and since 23.8
// `JobDetail.tiers`, `drone_cap`, `landing` and `approved_at`; and
// `LimitValues.concurrency` — "Drones at once", `settings.concurrency-cap`.
//
// **Classifying is the same status as #1159's `proposing`** (#1530, 21 Sep),
// and the workflow and the gates lock at **approval**, not at creation. So this
// is a live shape, editable right up to the approve press.
//
// **A proposal in flight is not a Job today**, which makes everything on the
// proposing screens draft twice over: the shape is draft, and so is the idea
// that it exists before a Job does.

import { ADVANCE_GATE, AUTO_MERGE, JOB_STATUS } from "@armada/components";
import type {
  DeclaredCheck,
  DeclaredJudge,
  JobDetail,
  WorkflowStep,
} from "@armada/protocol";

import type { PrMode } from "./landing";
import type { TaskTier } from "./task";

/** Which repository policy a step defers to, where it defers to one. */
export type RepositoryDecides = "auto_merge" | "review_gate";

/**
 * What the repository's policies say today, by the key that defers to them.
 * The word as `armada.yml` writes it — `human_always`, `checks-pass`;
 * `policyMeans` renders it.
 *
 * **Absent is a Fleet older than 18.4, never a default.** `ManifestSummary`
 * carries both words since then and a file declaring neither key crosses as
 * each policy's own default, so nothing else is missing — a reading handed
 * nothing says only that the repository decides.
 *
 * **One file's word and not the gate's answer.** Fleet folds a Job's several
 * gating Manifests at the gate and records nothing (`fleet::policy`), so this
 * is what the repository says today rather than what will happen.
 */
export type RepositorySays = Readonly<Partial<Record<RepositoryDecides, string>>>;

/**
 * What one step is gated by.
 *
 * **Four states per step, not three bits** (#1532, 22 Sep). Three tick boxes
 * cannot express a repository rule, so the fourth state is part of the type
 * rather than a flourish on the control — and it is not new to the wire:
 * `advance_gate` already carries `manifest_rule:auto_merge` and
 * `manifest_rule:review_gate` beside `auto`, `auto_if_judge_passes` and
 * `human_always`.
 */
export type GateView = {
  step_id: string;
  /** Whether the step's Checks are run at its gate. */
  checks: boolean;
  /** Whether the Judge looks. */
  judge: boolean;
  /** Whether it stops for you. */
  you: boolean;
  /** Which repository policy decides, where one does. Absent is the Job's own gate. */
  repository_decides?: RepositoryDecides;
  /** Whether this Job overrode the repository's rule for itself. */
  overridden?: boolean;
};

/**
 * Which model each tier runs on. `null` is the draft's spelling of a tier left
 * out, which on the wire (`TierModels`, 23.6) is a key left out: **Armada
 * picks, and the Drone's row says which model it ran** (spike 022, answer 8).
 */
export type TierModels = Readonly<Record<TaskTier, string | null>>;

/** A Job being classified, with everything that locks at approval. */
export type ProposalView = {
  /** `classifying`, and the same status #1159 calls `proposing`. */
  status: string;
  title: string;
  /**
   * The workflow this Job runs, by the id `WorkflowSummary.id` declares.
   *
   * **Editable up to the press, like everything else here** — the owner asked
   * for the picker on 28 Sep (`2b4j`), and what locks, locks at approval
   * (#1530). Changing it replaces every gate, because a gate belongs to a step
   * and the steps are the new workflow's: `gatesForSteps` is that rebuild.
   */
  workflow_id: string;
  /**
   * The request, in the requester's own words. `JobDetail.facts`, and absent
   * where a Job was given no context beyond its title.
   *
   * **Held here because it is editable before approval** (`d9b3`, 28 Sep).
   * Every Drone is given these words, so the last chance to fix them is the
   * gate; `facts` on the wire is a read and Fleet takes no rewrite of it.
   */
  asked?: string;
  gates: GateView[];
  /**
   * The line the ticks cannot turn off.
   *
   * **A step with nothing ticked still has Fleet checking the work stayed
   * inside the plan** (#1530, 22 Sep). It is `true` and never anything else —
   * a field rather than prose, so a screen cannot draw a step as ungated.
   */
  fleet_always_looks: true;
  tiers: TierModels;
  /**
   * How many Drones this Job may run at once. Absent is "as many as the
   * machine allows".
   */
  drone_cap?: number;
  /**
   * How many the machine allows, across every Job. `null` before Fleet said.
   *
   * **This is `LimitValues.concurrency`, and nothing on the wire is called
   * `machine_cap`.** Spike 022 recounts it in Drones in slice 5 with no change
   * of shape.
   */
  machine_cap: number | null;
  /** Where the work starts. See `LandingRule.from_ref`. */
  from_ref: string | null;
  pr_mode: PrMode;
  /** When it was approved. Absent is a proposal still being classified. */
  approved_at?: string;
};

/**
 * A Job at its approval gate, or approved, as Fleet serves it since 23.8.
 *
 * **`notes_for_planner` is not here.** It was dropped on 22 Sep 2026 — what
 * you want to say goes in the prompt.
 */
export function proposalViewOf(detail: JobDetail, machineCap: number | null): ProposalView {
  const view: ProposalView = {
    status: detail.job.status,
    title: detail.job.title,
    workflow_id: detail.job.workflow_id,
    gates: detail.steps.map(gateViewOf),
    fleet_always_looks: true,
    // A tier the map leaves out is Armada picking (spike 022, answer 8), which
    // is the draft's `null`.
    tiers: {
      difficult: detail.tiers?.difficult ?? null,
      medium: detail.tiers?.medium ?? null,
      easy: detail.tiers?.easy ?? null,
    },
    machine_cap: machineCap,
    // Absent is the Manifest's base, which is not a branch name to print.
    from_ref: detail.landing?.from_ref ?? null,
    pr_mode: detail.landing?.pr_mode === "draft" ? "draft" : "ready",
  };
  if (detail.facts !== undefined) view.asked = detail.facts;
  if (detail.drone_cap !== undefined) view.drone_cap = detail.drone_cap;
  if (detail.approved_at !== undefined) view.approved_at = detail.approved_at;
  return view;
}

/**
 * A gate per step of a workflow, as that workflow declares them.
 *
 * **What picking another workflow costs.** A gate belongs to a step, so the
 * gates a person moved on the old workflow name steps the new one does not
 * have; carrying them across by position would put a tick meant for `handoff`
 * on whatever runs fourth. So the rebuild is total, and the screen says so
 * beside the picker rather than silently discarding the ticks.
 */
export function gatesForSteps(steps: readonly WorkflowStep[]): GateView[] {
  return steps.map(gateViewOf);
}

/**
 * What a gate is read off, on a step Fleet holds and on one a workflow only
 * declares. `WorkflowSummary.steps` and `JobDetail.steps` carry the same four
 * fields with the same meanings, and this is the intersection rather than
 * either — a gate needs no other field, and taking the wider type would make
 * the workflow picker cast.
 */
export type GateDeclared = {
  step_id: string;
  checks?: readonly DeclaredCheck[];
  judge_checks?: readonly DeclaredJudge[];
  advance_gate?: string;
  overridden?: boolean;
};

/**
 * One step's gate, from its `advance_gate`.
 *
 * **Absent `advance_gate` is "Fleet cannot say"** — a Job naming a workflow
 * this Fleet does not hold — and it is not an ungated step. It reads as
 * stopping for a person, which is the answer that cannot advance work nobody
 * looked at.
 */
export function gateViewOf(step: GateDeclared): GateView {
  const gate = step.advance_gate;
  const view: GateView = {
    step_id: step.step_id,
    checks: (step.checks?.length ?? 0) > 0,
    judge: gate === "auto_if_judge_passes" || (step.judge_checks?.length ?? 0) > 0,
    you: gate === undefined || gate === "human_always",
  };
  if (gate === "manifest_rule:auto_merge") {
    view.repository_decides = "auto_merge";
  }
  if (gate === "manifest_rule:review_gate") {
    view.repository_decides = "review_gate";
  }
  if (view.repository_decides !== undefined) {
    view.overridden = step.overridden;
  }
  return view;
}

/**
 * What one gate is on the wire, and what Fleet does with it.
 *
 * **The boxes are not the wire and the wire is not the boxes.** `advance_gate`
 * is one value per step and says what it takes to *advance*; the Checks a step
 * runs and the criteria a Judge reads are declared on the step itself and are
 * frozen at creation (`crates/core-model/src/job/declared.rs`). So a tick
 * moves the gate and never the declarations — which is the whole of why
 * `unmeantOf` exists.
 */
export type GateReading = {
  /** The `advance_gate` this combination is today. */
  advance_gate: string;
  /** What Fleet does with it, in one sentence. */
  does: string;
};

/** What a step declares for the two automatic tiers to work on. */
export type Declared = { checks: boolean; judge: boolean };

/**
 * The status a person's gate holds a Job at, in the registry's own word.
 * **Never the wire's id** (owner, 1 Oct 2026, `#1748` row 16): the sentence
 * read `It holds at awaiting_review`; the sentence now reads `It reads needs review until you answer`, because the registry word is a predicate, Fleet's spelling where a person decides.
 */
const HOLDS_AT = JOB_STATUS["awaiting_review"]?.verb ?? "awaiting_review";

/** What the two `manifest_rule` keys decide, in the repository's own terms. */
const REPOSITORY_DOES: Readonly<Record<RepositoryDecides, string>> = {
  auto_merge:
    "The repository's auto_merge policy decides whether this lands without a person.",
  review_gate: "The repository's review_gate policy decides whether a person signs off.",
};

/**
 * What a repository's word for a policy means, in the verb generated for it,
 * or `undefined` where nothing read the word.
 *
 * **`review_gate` resolves to an `advance_gate`**, which is why its words come
 * from that table: `human_always` and `auto_if_judge_passes` are the two values
 * it resolves to, and each already renders as what it does to a step.
 */
function policyMeans(policy: RepositoryDecides, word: string | undefined): string | undefined {
  if (word === undefined) return undefined;
  // A word no registry renders reads as nothing, rather than as itself: a bare
  // `auto_if_judge_passes` in a sentence is Fleet's spelling where a person is
  // deciding, which is the reason `advanceGate` is held and not drawn.
  return (policy === "auto_merge" ? AUTO_MERGE[word] : ADVANCE_GATE[word])?.verb ?? undefined;
}

/**
 * One step's gate, read as what Fleet would do.
 *
 * The order is the order the wire resolves in: a repository rule nobody
 * overrode is the whole answer, then a person, then the two automatic tiers.
 * **A person outranks the other two boxes rather than replacing them** —
 * `HumanAlways` still runs the tiers, and what they establish is the material
 * the person reads.
 */
export function gateReadingOf(gate: GateView, says: RepositorySays = {}): GateReading {
  if (gate.repository_decides !== undefined && gate.overridden !== true) {
    const policy = gate.repository_decides;
    // What the deference resolves to, where the repository's word was read.
    // Deferred and unresolved is what nobody could read (`rhxt`, 29 Sep): the
    // policy can move, and this Job moves with it at every gate it reaches.
    const means = policyMeans(policy, says[policy]);
    return {
      advance_gate: `manifest_rule:${policy}`,
      does:
        means === undefined
          ? REPOSITORY_DOES[policy]
          : `Today that policy says ${means}. ` +
            "Editing the Manifest changes it, for this Job as well.",
    };
  }
  if (gate.repository_decides !== undefined) {
    // Overridden: the step keeps deferring on the record and the override wins
    // at every gate, however the repository's rule moves (spike 022, answer 4).
    const policy = gate.repository_decides;
    const means = policyMeans(policy, overrideWordOf(policy, gate));
    // The boxes are the gate again (#1548), and the sentence says what they
    // stand in for.
    if (means !== undefined) {
      return {
        advance_gate: gate.you ? "human_always" : gate.judge ? "auto_if_judge_passes" : "auto",
        does: `Overridden for this Job: ${means}, however the repository's ${policy} moves.`,
      };
    }
  }
  if (gate.you) {
    return {
      advance_gate: "human_always",
      does: ranBeside(`It reads ${HOLDS_AT} until you answer`, gate.checks, gate.judge),
    };
  }
  if (gate.judge) {
    return {
      advance_gate: "auto_if_judge_passes",
      does: gate.checks
        ? "Its Checks have to pass and the Judge has to decline to refuse them."
        : // There is no such thing as a Judge pass, only a mechanical pass a
          // Judge declined to refuse — so with no Check to fail, the Judge
          // refusing is the only thing that can hold it.
          "It advances unless the Judge refuses it.",
    };
  }
  return {
    advance_gate: "auto",
    does: gate.checks
      ? "Its Checks are the whole gate: they pass and it advances."
      : "Nothing stops it.",
  };
}

/**
 * The word an override stands in for the repository's, as Fleet lays it over
 * the fold (`approve_dispatch`, 23.8): `auto_merge` reads `never` with a person
 * ticked and `checks-pass` without one (owner, 2 Oct 2026); `review_gate` reads
 * `human_always` with a person and `auto_if_judge_passes` with the Judge.
 * `undefined` is a combination Fleet resolves to no word.
 */
function overrideWordOf(policy: RepositoryDecides, gate: GateView): string | undefined {
  if (policy === "auto_merge") return gate.you ? "never" : "checks-pass";
  if (gate.you) return "human_always";
  return gate.judge ? "auto_if_judge_passes" : undefined;
}

/**
 * What this combination asks for that Fleet has nothing to do, or `undefined`
 * where it is an ordinary gate.
 *
 * **A tick moves the gate and never what the step declares.** `mechanical_checks[]`
 * and `judge_checks[]` are the workflow's and are frozen at creation, so
 * asking for a Check on a step that declares none runs nothing at all — and
 * the combination reads, on the wire, exactly like one that does.
 */
export function unmeantOf(gate: GateView, declared: Declared): string | undefined {
  if (gate.checks && !declared.checks) {
    return "This step declares no Check, so asking for Checks runs nothing until the workflow declares one.";
  }
  if (gate.judge && !declared.judge) {
    return "This step declares nothing for a Judge to read, so asking for a Judge asks for a verdict on no criteria.";
  }
  return undefined;
}

/** What still runs on a step a person answers, where anything does. */
function ranBeside(said: string, checks: boolean, judge: boolean): string {
  const ran = [checks ? "its Checks" : undefined, judge ? "the Judge" : undefined].filter(
    (one) => one !== undefined,
  );
  return ran.length === 0
    ? `${said}, with nothing run before you read it.`
    : `${said}, with ${ran.join(" and ")} run first so you read what they found.`;
}

// The line the ticks cannot turn off used to be here, drawn once over the gate
// boxes. Fleet refuses work that went outside what the plan declared and looks
// for a gamed check whatever the boxes say (#1530, 22 Sep) —
// `crates/fleet/src/gate.rs`. It is true of a Job nobody approved, so it is
// guide 9 and the `?` on the region's heading (#1602).
