// An added step's repair or side run, read as the branch a Trigger's grows. **One drawing for both**:
// a Trigger's `JobTrigger` and an added step's `AddedStep` carry the same repair record, so the step is
// turned into the row the branch is drawn from, with the id the owner's choice names it by.

import type { AddedStep, JobTrigger } from "@armada/protocol";

/** A branch's row: a Trigger's own, or an added step turned into one. `addition` is the step's id. */
export type SideBranch = JobTrigger & { addition?: string };

/** What an added step is called: its command, its skill, or its brief. */
export const additionName = (one: AddedStep): string =>
  one.runs.kind === "script" ? one.runs.command : one.runs.kind === "skill" ? one.runs.skill : one.runs.brief;

/** The added steps a Drone has been put on, whether to repair a Script or to run a Skill or a Drone step, and a Script waiting on the owner to run. */
export function additionBranches(additions: readonly AddedStep[]): SideBranch[] {
  return additions.flatMap((one): SideBranch[] =>
    one.repair_record === undefined && one.state !== "awaiting_owner"
      ? []
      : [
          {
            name: additionName(one),
            when: one.when,
            step: one.step,
            level: "machine",
            state: one.state,
            ...(one.started_at === undefined ? {} : { started_at: one.started_at }),
            ...(one.log_at === undefined ? {} : { log_at: one.log_at }),
            ...(one.repair_record === undefined ? {} : { repair: one.repair_record }),
            ...(one.block ? { blocks: true } : {}),
            ...(one.runs.kind === "script" ? {} : { drone: true }),
            addition: one.id,
          },
        ],
  );
}

/** What names a branch's fix to Fleet: the Trigger's name, or the added step's id. */
export const fixOf = (one: SideBranch): { trigger: string } | { addition: string } =>
  one.addition === undefined ? { trigger: one.name } : { addition: one.addition };
