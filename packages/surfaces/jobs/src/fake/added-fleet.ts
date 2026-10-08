// What the mock's Fleet does with steps added to one Job, as `crates/fleet/src/added_steps.rs` does it
// (23.68): a place is checked against the frozen workflow and against where the Job is, a row is
// answered whole, removal works only before the step fires, and a Skill or a Drone step runs on a side
// Drone when its moment comes (23.73): `running` with a repair record, which the mock's repair Fleet moves on. Every shape is `packages/protocol/src/added-steps.ts`'s.

import type {
  AddedPlaced,
  AddedStep,
  AddStep,
  JobDetail,
  Outcome,
  TriggerScope,
} from "@armada/protocol";

const refusedAs = (code: string, message: string, fields: Record<string, string> = {}): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields, chain: [] },
});

/** A refusal or the row a call leaves. */
export type Added = { detail: JobDetail; added: AddedStep } | Outcome;

const textOf = (add: Pick<AddStep, "runs">): string =>
  add.runs.kind === "script" ? add.runs.command : add.runs.kind === "skill" ? add.runs.skill : add.runs.brief;

const delivering = (detail: JobDetail) => detail.steps.find((step) => step.delivers === true);

/** `placeable`: the step exists, and `pr_opened` hangs from the one that delivers. */
function misplaced(detail: JobDetail, add: AddStep): string | undefined {
  if (!detail.steps.some((step) => step.step_id === add.step)) return `the workflow has no step \`${add.step}\``;
  if (add.when === "pr_opened" && delivering(detail)?.step_id !== add.step) return `\`pr_opened\` hangs from the delivering step, which is not \`${add.step}\``;
  return undefined;
}

/** `behind`: whether the moment has already come on a Job underway. */
function behind(detail: JobDetail, add: AddStep): Outcome | undefined {
  const status = detail.job.status;
  const where = { when: add.when, step: add.step };
  if (status === "proposing" || status === "awaiting_approval") {
    return refusedAs("fleet.added_step_before_approval", "this Job is not approved, so a step added to it goes with the approval", where);
  }
  if (["completed_failed", "completed_success", "killed", "rejected", "superseded"].includes(status)) {
    return refusedAs("fleet.added_step_behind", "this Job is over, so there is nothing left to add a step to", { ...where, reason: "ended" });
  }
  const row = detail.steps.find((step) => step.step_id === add.step)!;
  const current = detail.steps.find((step) => step.step_id === detail.job.current_step_id);
  if (add.when !== "step_passes") {
    return row.state === "not_started"
      ? undefined
      : refusedAs("fleet.added_step_behind", `step \`${add.step}\` has already started, so a step cannot go before it`, { ...where, reason: "started" });
  }
  const past = row.state === "advanced" || (current !== undefined && row.ordinal < current.ordinal);
  return past ? refusedAs("fleet.added_step_behind", `the Job is past step \`${add.step}\`, so a step cannot go after it`, { ...where, reason: "passed" }) : undefined;
}

function rowOf(add: AddStep, id: string, placed: AddedPlaced, at: string): AddedStep {
  return {
    id,
    runs: add.runs,
    when: add.when,
    step: add.step,
    block: add.block === true,
    repair: add.repair === true,
    placed,
    added_at: at,
    state: "pending",
  };
}

const nextId = (rows: readonly AddedStep[]) => `a${rows.reduce((most, one) => Math.max(most, Number(one.id.slice(1)) || 0), 0) + 1}`;

/** `add_job_step`. */
export function addedTo(detail: JobDetail, add: AddStep, at: string): Added {
  if (textOf(add).trim() === "") return refusedAs("fleet.unacceptable_addition", "an added step names nothing to run");
  const wrong = misplaced(detail, add);
  if (wrong !== undefined) return refusedAs("fleet.unacceptable_addition", wrong);
  const late = behind(detail, add);
  if (late !== undefined) return late;
  const rows = detail.additions ?? [];
  const added = rowOf(add, nextId(rows), "running", at);
  return { detail: { ...detail, additions: [...rows, added] }, added };
}

/** The additions an approval places, as `approve_dispatch` records them. */
export function placedBy(adds: readonly AddStep[], at: string): AddedStep[] {
  return adds.reduce<AddedStep[]>((rows, add) => [...rows, rowOf(add, nextId(rows), "approval", at)], []);
}

/** `remove_job_step`, only before the step fires. The row stays in Fleet's record and leaves the Job's list. */
export function removedFrom(detail: JobDetail, id: string): { detail: JobDetail } | Outcome {
  const row = (detail.additions ?? []).find((one) => one.id === id);
  if (row === undefined) return refusedAs("fleet.no_such_addition", `this Job holds no addition \`${id}\``, { id });
  if (row.state !== "pending") return refusedAs("fleet.added_step_fired", `\`${id}\` has fired, so it is part of what this Job did and stays`, { id });
  return { detail: { ...detail, additions: (detail.additions ?? []).filter((one) => one.id !== id) } };
}

/** `edit_job_step`, only before the step fires; a switch left out is left as it is. */
export function editedIn(detail: JobDetail, id: string, next: { block?: boolean; repair?: boolean }): { detail: JobDetail; edited: AddedStep } | Outcome {
  const row = (detail.additions ?? []).find((one) => one.id === id);
  if (row === undefined) return refusedAs("fleet.no_such_addition", `this Job holds no addition \`${id}\``, { id });
  if (row.state !== "pending") return refusedAs("fleet.added_step_fired", `\`${id}\` has fired, so it is part of what this Job did and stays as it was`, { id });
  const edited = { ...row, block: next.block ?? row.block, repair: next.repair ?? row.repair };
  return { detail: { ...detail, additions: (detail.additions ?? []).map((one) => (one.id === id ? edited : one)) }, edited };
}

/** Where an addition was kept, once the Trigger it became is written. */
export function keptAs(detail: JobDetail, id: string, scope: TriggerScope): JobDetail {
  return { ...detail, additions: (detail.additions ?? []).map((one) => (one.id === id ? { ...one, kept: scope } : one)) };
}

/** A moment passing: each pending step is reached, and each running Script ends. A Skill and a Drone step get a Drone and stay running until it answers; a Script the repository does not declare is skipped. */
export function advanced(detail: JobDetail, at: string, declared: readonly string[]): JobDetail | undefined {
  const rows = detail.additions ?? [];
  if (rows.every((one) => one.state === "passed" || one.state === "skipped" || one.state === "failed" || one.repair_record !== undefined)) return undefined;
  return {
    ...detail,
    additions: rows.map((one): AddedStep => {
      if (one.state === "pending") {
        if (one.runs.kind !== "script") {
          return { ...one, state: "running", started_at: at, repair_record: { attempt: 1, branch: `armada/run-${one.id}-1` } };
        }
        if (!declared.includes(one.runs.command)) {
          return { ...one, state: "skipped", skipped: { reason: "not_in_this_repo", said: `skipped: \`${one.runs.command}\` is not in this repo` }, ended_at: at };
        }
        return { ...one, state: "running", started_at: at };
      }
      return one.state === "running" && one.repair_record === undefined ? { ...one, state: "passed", exit_code: 0, ended_at: at } : one;
    }),
  };
}
