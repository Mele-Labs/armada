// The seven other workflows, with the steps their own files declare.
//
// **Read off `.armada/workflows/*.json`, never drawn.** Each of these carries
// that file's step ids, labels, gates and Judge counts — `bug` is three steps
// and not six, `revert` is two, and there is no `verify-and-ship` because no
// file declares one (#1530, 22 Sep).
//
// **`checks` is the caller's**, because which Manifest Checks a step runs
// depends on what the Job writes: `armada.yml` selects them by `when:` glob,
// so a Job touching Rust and a Job touching Bridge run different sets through
// the same step of the same workflow.

import type { DeclaredCheck, WorkflowSummary } from "@armada/protocol";
import { MANIFEST_ID } from "./base";
import type { StepPhase } from "@armada/protocol";

type Step = WorkflowSummary["steps"][number];

function step(
  step_id: string,
  label: string,
  advance_gate: string,
  over: Partial<Step> = {},
  /** Its lane on the approval canvas. Absent is Fleet's own reading. */
  phase?: StepPhase,
): Step {
  const declared: Step = {
    step_id,
    label,
    checks: [],
    judge_checks: [],
    advance_gate,
    delivers: false,
    ...over,
  };
  return phase === undefined ? declared : { ...declared, phase };
}

function summary(id: string, name: string, steps: Step[]): WorkflowSummary {
  return { id, name, version: 1, manifest_id: MANIFEST_ID, steps };
}

export function bugWorkflow(checks: DeclaredCheck[]): WorkflowSummary {
  return summary("bug", "bug", [
    step("plan", "Plan the change", "auto_if_judge_passes", {
      judge_checks: [{ criteria: 2, gaming_check: false }],
    }, "work"),
    step("implement", "Implement", "auto_if_judge_passes", {
      checks,
      judge_checks: [{ criteria: 4, gaming_check: true }],
    }, "work"),
    step("handoff", "Review the change", "human_always", { delivers: true }, "delivery"),
  ]);
}

export function refactorWorkflow(checks: DeclaredCheck[]): WorkflowSummary {
  return summary("refactor", "refactor", [
    step("plan", "Scope the refactor", "auto_if_judge_passes", {
      judge_checks: [{ criteria: 1, gaming_check: false }],
    }, "work"),
    step("implement", "Restructure", "auto_if_judge_passes", {
      checks,
      judge_checks: [{ criteria: 4, gaming_check: true }],
    }, "work"),
    step("handoff", "Review the change", "human_always", { delivers: true }, "delivery"),
  ]);
}

export function revertWorkflow(checks: DeclaredCheck[]): WorkflowSummary {
  return summary("revert", "revert", [
    step("revert", "Undo the change", "auto_if_judge_passes", {
      checks,
      judge_checks: [{ criteria: 2, gaming_check: false }],
    }),
    step("handoff", "Summarise", "human_always", { delivers: true }),
  ]);
}

export function prototypeWorkflow(): WorkflowSummary {
  return summary("prototype", "prototype", [
    step("frame", "Frame", "auto"),
    step("build", "Build", "human_always"),
    step("write_up", "Write up", "auto"),
  ]);
}

export function designPlanWorkflow(): WorkflowSummary {
  return summary("design_plan", "design_plan", [
    step("draft", "Draft", "auto"),
    step("present", "Present", "human_always"),
  ]);
}

export function codeReviewWorkflow(): WorkflowSummary {
  return summary("code_review", "code_review", [
    step("read", "Read the diff", "auto"),
    step("assess", "Assess", "human_always"),
    step("deliver", "Deliver the review", "auto"),
  ]);
}

export function epicWorkflow(): WorkflowSummary {
  return summary("epic", "epic", [
    step("plan", "Plan the wave", "human_always", {
      judge_checks: [{ criteria: 2, gaming_check: false }],
    }, "work"),
    // The roll-up delivers no change, and is what the wave delivers.
    step("roll_up", "Roll up the wave", "human_always", {}, "delivery"),
  ]);
}
