// When Pause and Resume are offered, what the mark says, and what each refusal
// reads as. These are the rules the Board, Job detail and Cleanup draw from.

import { describe, expect, it } from "vitest";

import type { JobSummary, Outcome, WorktreeHeld } from "@armada/protocol";
import { canPause, canResume, pauseFactsOf, pauseRefusal, pausedSaid, refusedAsPaused } from "./pausing";

const NOW = Date.parse("2026-10-06T10:00:00Z");

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01JOB",
    handle: "2-debounce",
    title: "Debounce the Job Board",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01MAN",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-10-06T09:00:00Z",
    branch: "armada/2-debounce",
    ...over,
  };
}

const paused = (over: Partial<NonNullable<JobSummary["paused"]>> = {}) => ({ by: "person", at: "2026-10-06T09:56:00Z", resuming: false, ...over });

const refused = (code: string, message = "git said no"): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "r", fields: {}, chain: [] },
});

describe("canPause", () => {
  it.each(["running", "awaiting_review", "awaiting_repair", "escalated"])("is offered on a %s Job", (status) => {
    expect(canPause(job({ status }))).toBe(true);
  });

  it.each(["awaiting_approval", "completed_success", "killed", "proposing"])("is not offered on a %s Job", (status) => {
    expect(canPause(job({ status }))).toBe(false);
  });

  it("is offered on a queued Job only where its slot is known", () => {
    expect(canPause(job({ status: "queued" }))).toBe(false);
    expect(canPause(job({ status: "queued" }), { slot: true })).toBe(true);
  });

  it("is not offered on a paused Job, or while its Checks run", () => {
    expect(canPause(job({ paused: paused() }))).toBe(false);
    expect(canPause(job(), { checksRunning: true })).toBe(false);
  });
});

describe("canResume", () => {
  it("is offered on a paused Job and on no other", () => {
    expect(canResume(job({ paused: paused() }))).toBe(true);
    expect(canResume(job())).toBe(false);
  });

  it("is not offered once the resume is waiting for a slot, which the same press would only repeat", () => {
    expect(canResume(job({ paused: paused({ resuming: true }) }))).toBe(false);
  });
});

describe("pausedSaid", () => {
  it("names the branch the work is on, and when", () => {
    expect(pausedSaid(job({ paused: paused() }), NOW)).toBe("Paused 4 minutes ago: work saved on branch armada/2-debounce, slot released");
  });

  it("names Fleet where Fleet paused it", () => {
    expect(pausedSaid(job({ paused: paused({ by: "fleet" }) }), NOW)).toBe(
      "Paused by Fleet 4 minutes ago: work saved on branch armada/2-debounce, slot released",
    );
  });

  it("says the slot is not back while a resume waits", () => {
    expect(pausedSaid(job({ paused: paused({ resuming: true }) }), NOW)).toBe(
      "Paused 4 minutes ago: waiting for a slot to resume branch armada/2-debounce",
    );
  });

  it("says nothing of a Job that is not paused", () => {
    expect(pausedSaid(job(), NOW)).toBeUndefined();
  });
});

describe("pauseFactsOf", () => {
  const held = (files: string[]): WorktreeHeld => ({
    job_id: "01JOB",
    job_title: "Debounce the Job Board",
    status: "running",
    last_moved_at: "2026-10-06T09:50:00Z",
    path: "/Users/user/armada/.armada/slots/slot-3",
    branch: "armada/2-debounce",
    held: files.length === 0 ? [] : [{ why: "uncommitted", files }],
    on_disk: true,
  });

  it("lists the uncommitted files and the slot, and says the Drone stops, on a running Job", () => {
    expect(pauseFactsOf(job(), held(["src/a.rs"]))).toEqual({ branch: "armada/2-debounce", files: ["src/a.rs"], slot: "slot-3", running: true });
  });

  it("has no process to stop on a Job parked at a gate", () => {
    expect(pauseFactsOf(job({ status: "awaiting_review" }), held([])).running).toBe(false);
  });

  it("names no files and no slot where the read has not arrived", () => {
    expect(pauseFactsOf(job(), undefined)).toEqual({ branch: "armada/2-debounce", files: [], running: true });
  });
});

describe("refusals", () => {
  it.each([
    ["pause_job", "fleet.not_pausable", "Not paused: no worktree to give back"],
    ["pause_job", "fleet.already_paused", "Already paused"],
    ["resume_job", "fleet.not_paused", "Not paused"],
    ["pause_job", "fleet.checks_running", "Not paused: its Checks are reading the worktree"],
    ["pause_job", "fleet.pause_refused", "Not paused: git said no"],
    ["resume_job", "fleet.pause_refused", "Not resumed: git said no"],
  ] as const)("%s refused as %s reads %s", (act, code, said) => {
    expect(pauseRefusal(act, refused(code))).toBe(said);
  });

  it("leaves any other refusal to the pipeline every refusal goes through", () => {
    expect(pauseRefusal("pause_job", refused("fleet.frozen"))).toBeUndefined();
    expect(pauseRefusal("pause_job", { ok: true })).toBeUndefined();
  });

  it("knows an act refused because the Job is paused", () => {
    expect(refusedAsPaused(refused("fleet.paused"))).toBe(true);
    expect(refusedAsPaused(refused("fleet.not_paused"))).toBe(false);
    expect(refusedAsPaused({ ok: true })).toBe(false);
  });
});
