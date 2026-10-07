// The Triggers frozen onto Job 2, one in each state a firing can be in, shaped exactly as
// `JobDetail.triggers` carries them (`packages/protocol/src/triggers.ts`) and with the log line
// Fleet writes for each (`fleet::triggering::firing_line`): stamped with the firing's own end,
// or its start where it has none, and naming the Trigger in its `trigger` field.
//
// Invented: Job 2 was recorded before Triggers, so nothing here is something Fleet served.

import type { JobTrigger, Noted } from "@armada/protocol";

export const at = (second: number) => `2026-10-01T21:2${Math.floor(second / 60)}:${String(second % 60).padStart(2, "0")}.000Z`;

/** Every state, in the order a Job meets the moments: `implement` passes, then `handoff` opens its pull request. */
export const JOB_2_TRIGGERS: JobTrigger[] = [
  { name: "fmt", when: "step_passes", step: "implement", level: "repository", state: "passed", exit_code: 0, started_at: at(10), ended_at: at(12), log_at: at(12) },
  { name: "gate", when: "pr_opened", step: "handoff", level: "machine", state: "failed", exit_code: 1, started_at: at(20), ended_at: at(31), log_at: at(31) },
  {
    name: "deploy_qa",
    when: "pr_opened",
    step: "handoff",
    level: "machine",
    state: "skipped",
    skipped: { reason: "not_in_this_repo", name: "deploy_qa", said: "`deploy_qa` is not a Command this repository declares" },
    started_at: at(20),
    ended_at: at(20),
    log_at: at(20),
  },
  { name: "wipe_qa", when: "pr_opened", step: "handoff", level: "repository", state: "awaiting_owner", started_at: at(21), log_at: at(21) },
  { name: "smoke", when: "step_starts", step: "handoff", level: "repository", state: "running", started_at: at(22) },
  { name: "lint_docs", when: "step_passes", step: "handoff", level: "repository", state: "pending" },
];

const SAID: Record<JobTrigger["state"], string> = {
  pending: "pending",
  skipped: "skipped",
  running: "running",
  passed: "passed",
  failed: "failed",
  awaiting_owner: "awaiting_owner",
  repairing: "repairing",
  rerunning: "rerunning",
  fix_ready: "fix_ready",
};

/** The log's line for each firing that has one, after the notes already in the log. */
export function triggerNotes(after: number, triggers: readonly JobTrigger[] = JOB_2_TRIGGERS): Noted[] {
  return triggers.flatMap((one, i): Noted[] => {
    if (one.log_at === undefined) return [];
    return [
      {
        seq: after + i + 1,
        at: one.log_at,
        by: "fleet",
        level: one.state === "failed" ? "warn" : "info",
        msg:
          one.state === "awaiting_owner"
            ? `Trigger \`${one.name}\` is on a destructive Command and waits on you before it runs`
            : one.state === "skipped"
              ? `Trigger \`${one.name}\` ${one.skipped?.said ?? "was skipped"}`
              : `Trigger \`${one.name}\` ${SAID[one.state]}`,
        step: one.step,
        fields: [
          { name: "trigger", value: one.name },
          { name: "when", value: one.when },
          { name: "step", value: one.step },
          { name: "source", value: one.level },
          { name: "state", value: one.state },
          ...(one.exit_code === undefined ? [] : [{ name: "exit_code", value: String(one.exit_code) }]),
        ],
      },
    ];
  });
}
