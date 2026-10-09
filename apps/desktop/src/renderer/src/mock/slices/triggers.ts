// Triggers' members: the mock Fleet's four routes, answered from `triggers-fleet.ts`, and a failed
// Trigger's repair, answered from `repair-fleet.ts`. Steps added to one Job are here too, answered
// from `added-fleet.ts` as Fleet answers them, on the Job open.
// **A step is reached and ends when a walk's `later` says time has passed**, so what a walk looks
// at is the same on every run.

import { addedTo, advanced, editedIn, keptAs, removedFrom } from "@armada/jobs/fake";
import { DECLARED } from "@armada/manifest/fake";

import type { JobDiff } from "@armada/protocol";
import type { AddedStepsApi } from "../../../../shared/api/added-steps";
import type { TriggersApi, TriggersState } from "../../../../shared/api/triggers";
import { TRIGGERS_NOTHING_YET } from "../../../../shared/api/triggers";
import type { Fleet, Slice } from "../fake-context";
import { holdServed } from "../hold-fleet";
import { repairServed } from "../repair-fleet";
import { alsoOnTimePassing } from "../time-passes";
import { triggersServed } from "../triggers-fleet";

/** A fix's patch as the mock reads it: each file gains one line, so a file opens to something. */
function repairDiffOf(jobId: string, branch: string | undefined, files: readonly string[]): JobDiff {
  const patch = files.map((path) => `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n@@ -1,2 +1,3 @@\n context\n-before the fix\n+after the fix\n+and one more line\n`).join("");
  return {
    job_id: jobId,
    work: { files: files.map((path) => ({ path, change: "modified" })), ...(branch === undefined ? {} : { measured_from: branch }), measured_whole: true, plan_declared: false, ...(patch === "" ? {} : { patch }) },
  };
}

const none = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

/** The detail of the Job open, where it is the one named. */
function openOn(fleet: Fleet, jobId: string) {
  const watched = fleet.state().watched;
  return watched.state === "read" && watched.jobId === jobId ? watched.detail : undefined;
}

function served(fleet: Fleet): Omit<TriggersApi, "chooseTriggerFix" | "rerunTrigger" | "skipTrigger"> & AddedStepsApi {
  const triggers = triggersServed();
  alsoOnTimePassing(() => {
    const watched = fleet.state().watched;
    if (watched.state === "read") fleet.stepping(watched.jobId, (whole, at) => advanced(
        whole,
        at,
        DECLARED.commands.map((one) => one.name),
        DECLARED.commands.filter((one) => one.command.destructive === true).map((one) => one.name),
      ));
  });
  return {
    ...triggers,
    // Read off the Jobs the Fleet holds, as Fleet derives it: a Job with an alert is listed, `blocked` where it is
    // stopped mid-flight, and nothing here depends on which Job is open.
    readAlerts: async () => {
      const rows = fleet.state().jobs.filter((job) => job.alert !== undefined);
      const alert = (job: (typeof rows)[number]) => ({ job_id: job.id, handle: job.handle, status: job.status });
      return {
        ok: true,
        blocked: rows.filter((job) => job.status === "escalated").map(alert),
        waiting: rows.filter((job) => job.status !== "escalated").map(alert),
      };
    },
    // A save that keeps an addition says where it went on the Job's own row.
    saveTrigger: async (saving) => {
      const answer = await triggers.saveTrigger(saving);
      const from = saving.body.kept_from;
      if (answer.ok && from !== undefined) fleet.stepping(from.job_id, (whole) => keptAs(whole, from.addition_id, saving.body.scope));
      return answer;
    },
    addJobStep: async ({ jobId, body }) => {
      const whole = openOn(fleet, jobId);
      if (whole === undefined) return none;
      const answer = addedTo(whole, body, new Date().toISOString());
      if ("ok" in answer) return { ok: false, outcome: answer };
      fleet.stepping(jobId, () => answer.detail);
      return { ok: true, added: answer.added };
    },
    editJobStep: async ({ jobId, body }) => {
      const whole = openOn(fleet, jobId);
      if (whole === undefined) return none;
      const answer = editedIn(whole, body.id, body);
      if ("ok" in answer) return { ok: false, outcome: answer };
      fleet.stepping(jobId, () => answer.detail);
      return { ok: true, edited: answer.edited };
    },
    readRepairDiff: async ({ jobId, of }) => {
      const whole = openOn(fleet, jobId);
      if (whole === undefined) return none;
      const repair = of.addition === undefined ? (whole.triggers ?? []).find((one) => one.name === of.trigger)?.repair : (whole.additions ?? []).find((one) => one.id === of.addition)?.repair_record;
      return { ok: true, diff: repairDiffOf(jobId, whole.job.branch, repair?.files ?? []) };
    },
    removeJobStep: async ({ jobId, body }) => {
      const whole = openOn(fleet, jobId);
      if (whole === undefined) return none;
      const answer = removedFrom(whole, body.id);
      if ("ok" in answer) return { ok: false, outcome: answer };
      fleet.stepping(jobId, () => answer.detail);
      return { ok: true, removed: { id: body.id } };
    },
  };
}

export const triggers: Slice<TriggersApi & AddedStepsApi, TriggersState> = {
  name: "triggers",
  state: TRIGGERS_NOTHING_YET,
  api: (_scenario, fleet) => ({ ...served(fleet), ...repairServed(fleet), ...holdServed(fleet) }),
};
