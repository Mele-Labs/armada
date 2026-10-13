// The fixes a Job's call card offers, each one Fleet's route: words go to the Drone that is live
// (`redirect_one_drone`) or, where none is, start the step again with them (`restart_step`); Retry step
// is `restart_step`; Retry, Skip and Skip all on a Check are `retry_check`, `skip_check` and `skip_checks`.

import type { Outcome } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

export type JobFix =
  | { kind: "retry_check" | "skip_check"; jobId: string; check: string }
  | { kind: "skip_checks"; jobId: string }
  | { kind: "restart_step"; jobId: string; words?: string }
  | { kind: "redirect_one_drone"; jobId: string; words: string; droneId?: string };

/** Fleet's answer to a fix: taken, or refused in these words. */
export type JobFixed = { ok: true } | { ok: false; said: string };

/** A refusal that says no Drone is live to be told, which is when restarting the step is what applies. */
const NO_LIVE_DRONE = new Set(["fleet.drone_not_live", "fleet.illegal_move"]);
const noLiveDrone = (outcome: Outcome): boolean => !outcome.ok && outcome.why === "refused" && NO_LIVE_DRONE.has(outcome.error.code);

const told = (outcome: Outcome): JobFixed => (outcome.ok ? { ok: true } : { ok: false, said: refusalWords(outcome) });

export async function fixJob(fix: JobFix): Promise<JobFixed> {
  const api = window.armada;
  switch (fix.kind) {
    case "restart_step":
      return told(await api.restartStep(fix.jobId, fix.words));
    case "redirect_one_drone": {
      const sent = await api.redirectDrone(fix.jobId, fix.words, fix.droneId);
      return told(noLiveDrone(sent) ? await api.restartStep(fix.jobId, fix.words) : sent);
    }
    case "retry_check":
      return told(await api.retryCheck(fix.jobId, fix.check));
    case "skip_check":
      return told(await api.skipCheck(fix.jobId, fix.check));
    case "skip_checks":
      return told(await api.skipChecks(fix.jobId));
  }
}
