// Pause and Resume as a person meets them: when each is offered, what the mark
// beside the badge says, what the pause confirm lists, and what each refusal
// reads as. `docs/concepts/job.md`, *Pausing a Job*, has the model.

import type { PauseFacts } from "@armada/components";
import type { JobSummary, Outcome, WorktreeHeld } from "@armada/protocol";

import { costOf, sitting, slotNameOf } from "./held";

/** The two acts, as `ACT_LABEL` and the confirm name them. */
export type PauseAct = "pause_job" | "resume_job";

/** Statuses that can hold a pause: a Job that has a worktree to give back. `queued` needs a slot too. */
const PAUSABLE: readonly string[] = ["running", "awaiting_review", "awaiting_repair", "escalated"];

/**
 * Whether Pause is offered. **A queued Job qualifies only where its slot is
 * known**: a row does not carry one, so the Board never offers it there and the
 * Cleanup grid, which reads the pool, does. A paused Job is not offered it
 * again, and none is offered while its Checks run, which Fleet refuses.
 */
export function canPause(job: JobSummary, known: { slot?: boolean; checksRunning?: boolean } = {}): boolean {
  if (job.paused !== undefined || known.checksRunning === true) return false;
  return PAUSABLE.includes(job.status) || (job.status === "queued" && known.slot === true);
}

/** Whether Resume is offered: a paused Job, unless its resume is already waiting for a slot. */
export function canResume(job: JobSummary): boolean {
  return job.paused !== undefined && !job.paused.resuming;
}

/**
 * What the mark's tooltip reads: who paused it, when, and where the work is.
 * A resume waiting on the pool says that instead, since the slot is not back.
 * **Without `now` it says no time**, for a surface memoised across ticks.
 */
export function pausedSaid(job: JobSummary, now?: number): string | undefined {
  const { paused } = job;
  if (paused === undefined) return undefined;
  const by = paused.by === "fleet" ? "Paused by Fleet" : "Paused";
  const when = now === undefined ? null : sitting(paused.at, now);
  const lead = when === null ? by : `${by} ${when} ago`;
  const branch = job.branch === undefined ? "its branch" : `branch ${job.branch}`;
  return paused.resuming ? `${lead}: waiting for a slot to resume ${branch}` : `${lead}: work saved on ${branch}, slot released`;
}

/**
 * What the pause confirm lists. **Files and the slot come from the held read**
 * where it has arrived, and the lines for them stay out where it has not: a
 * confirm that named a commit the worktree may not need would claim a cost.
 */
export function pauseFactsOf(job: JobSummary, held: WorktreeHeld | undefined): PauseFacts {
  const slot = held === undefined ? null : slotNameOf(held.path);
  return {
    branch: job.branch ?? held?.branch ?? "its branch",
    files: held === undefined ? [] : costOf(held).files,
    ...(slot === null ? {} : { slot }),
    running: job.status === "running",
  };
}

/** Whether Fleet turned an act away because the Job is paused, which is the confirm to resume. */
export function refusedAsPaused(outcome: Outcome): boolean {
  return !outcome.ok && outcome.why === "refused" && outcome.error.code === "fleet.paused";
}

const NOT: Record<PauseAct, string> = { pause_job: "Not paused", resume_job: "Not resumed" };

/**
 * What one refusal of Pause or Resume reads as, in git's words, led by what did
 * not happen. `fleet.pause_refused` carries Fleet's own reason, which is git's
 * or the pool's. `undefined` for anything else, which draws as every other
 * refusal does.
 */
export function pauseRefusal(act: PauseAct, outcome: Outcome): string | undefined {
  if (outcome.ok || outcome.why !== "refused") return undefined;
  const { code, message } = outcome.error;
  switch (code) {
    case "fleet.not_pausable":
      return `${NOT[act]}: no worktree to give back`;
    case "fleet.already_paused":
      return "Already paused";
    case "fleet.not_paused":
      return "Not paused";
    case "fleet.checks_running":
      return `${NOT[act]}: its Checks are reading the worktree`;
    case "fleet.pause_refused":
      return `${NOT[act]}: ${message}`;
    default:
      return undefined;
  }
}
