// Triggers' members: the mock Fleet's four routes, answered from `triggers-fleet.ts`. Steps added to
// one Job are here too, answered from `added-fleet.ts` as Fleet answers them, on the Job open.
// **A step is reached and ends when a walk's `later` says time has passed**, so what a walk looks
// at is the same on every run.

import { addedTo, advanced, keptAs, removedFrom } from "@armada/jobs/fake";
import { DECLARED } from "@armada/manifest/fake";

import type { AddedStepsApi } from "../../../../shared/api/added-steps";
import type { TriggersApi, TriggersState } from "../../../../shared/api/triggers";
import { TRIGGERS_NOTHING_YET } from "../../../../shared/api/triggers";
import type { Fleet, Slice } from "../fake-context";
import { alsoOnTimePassing } from "../time-passes";
import { triggersServed } from "../triggers-fleet";

const none = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

/** The detail of the Job open, where it is the one named. */
function openOn(fleet: Fleet, jobId: string) {
  const watched = fleet.state().watched;
  return watched.state === "read" && watched.jobId === jobId ? watched.detail : undefined;
}

function served(fleet: Fleet): TriggersApi & AddedStepsApi {
  const triggers = triggersServed();
  alsoOnTimePassing(() => {
    const watched = fleet.state().watched;
    if (watched.state === "read") fleet.stepping(watched.jobId, (whole, at) => advanced(whole, at, DECLARED.commands.map((one) => one.name)));
  });
  return {
    ...triggers,
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
  api: (_scenario, fleet) => served(fleet),
};
