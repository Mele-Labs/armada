// The fixes a Job's call card offers, as Fleet's routes will take them: `retry_check`, `skip_check`,
// `skip_checks`, `redirect_one_drone` and `restart_step`. **Mock only until those routes land**: it
// answers after a beat and reaches nothing, so the card shows its pending state and leaves.

export type JobFix =
  | { kind: "retry_check" | "skip_check"; jobId: string; check: string }
  | { kind: "skip_checks"; jobId: string }
  | { kind: "restart_step"; jobId: string; words?: string }
  | { kind: "redirect_one_drone"; jobId: string; words: string };

/** Fleet's answer to a fix: taken, or refused in these words. */
export type JobFixed = { ok: true } | { ok: false; said: string };

const BEAT_MS = 350;

export const fixJob = (_fix: JobFix): Promise<JobFixed> => new Promise((resolve) => setTimeout(() => resolve({ ok: true }), BEAT_MS));
