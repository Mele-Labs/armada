// The fixes a Job's call card offers. **Message and Retry step are Fleet's**: words go to the Drone
// that is live (`redirect_one_drone`) or, where none is, start the step again with them
// (`restart_step`). **Retry, Skip and Skip all on a Check have no route on main** (`retry_check`,
// `skip_check` and `skip_checks` are on Job 14's branch), so the card draws those keys only where
// `checkFixesOf` finds them: the mock registers a stand-in beside its api, the real preload none.

import type { Outcome } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

export type JobFix =
  | { kind: "retry_check" | "skip_check"; jobId: string; check: string }
  | { kind: "skip_checks"; jobId: string }
  | { kind: "restart_step"; jobId: string; words?: string }
  | { kind: "redirect_one_drone"; jobId: string; words: string; droneId?: string };

/** Fleet's answer to a fix: taken, or refused in these words. */
export type JobFixed = { ok: true } | { ok: false; said: string };

/** The three Check fixes, as Job 14 will put them on the preload. */
export type CheckFixes = {
  retryCheck: (jobId: string, check: string) => Promise<Outcome>;
  skipCheck: (jobId: string, check: string) => Promise<Outcome>;
  skipChecks: (jobId: string) => Promise<Outcome>;
};

/** Beside the api rather than on it: the preload has no such member to type it by (as `fake.ts`' drafts). */
const CHECK_FIXES = new WeakMap<object, CheckFixes>();

/** A stand-in for routes main does not have, for the mock to register on its own api. */
export const registerCheckFixes = (api: object, fixes: CheckFixes): void => void CHECK_FIXES.set(api, fixes);

/** The Check fixes this window's Fleet answers, where it answers any. */
export const checkFixesOf = (api: object = window.armada): CheckFixes | undefined => CHECK_FIXES.get(api);

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
    default: {
      const fixes = checkFixesOf(api);
      if (fixes === undefined) return { ok: false, said: "Fleet cannot do that yet" };
      if (fix.kind === "skip_checks") return told(await fixes.skipChecks(fix.jobId));
      return told(await (fix.kind === "retry_check" ? fixes.retryCheck : fixes.skipCheck)(fix.jobId, fix.check));
    }
  }
}
