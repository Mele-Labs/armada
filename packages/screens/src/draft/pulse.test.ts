// One worktree becomes a list, and a process gains an owner.

import type { JobProcess, JobResources } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { pulseViewOf } from "./pulse";

function process(over: Partial<JobProcess> = {}): JobProcess {
  return {
    pid: 4121,
    command: "node",
    cpu_percent: 12.5,
    memory_bytes: 184_000_000,
    running_for: "04:12",
    recorded: true,
    ...over,
  };
}

function resources(over: Partial<JobResources> = {}): JobResources {
  return {
    job_id: "01J",
    read_at: "2026-09-22T10:30:00Z",
    held: "running",
    processes: [process()],
    worktree: { path: "/repo/.armada/worktrees/1532", branch: "armada/1532-draft-schema" },
    ...over,
  };
}

describe("the worktrees a Job holds", () => {
  it("is a list of the one the wire carries", () => {
    const view = pulseViewOf(resources());

    expect(view.worktrees).toHaveLength(1);
    expect(view.worktrees[0]?.branch).toBe("armada/1532-draft-schema");
  });

  it("is empty where the Job has no checkout, never a row with blank fields", () => {
    const bare = resources();
    delete bare.worktree;

    expect(pulseViewOf(bare).worktrees).toEqual([]);
  });
});

describe("who owns a process", () => {
  it("is the one worktree's branch, because that is all a Job has today", () => {
    expect(pulseViewOf(resources()).processes[0]?.owner).toBe("armada/1532-draft-schema");
  });

  it("is null where there is no worktree to place it in", () => {
    const bare = resources();
    delete bare.worktree;

    expect(pulseViewOf(bare).processes[0]?.owner).toBeNull();
  });

  it("keeps the process named by its command and never its arguments", () => {
    const view = pulseViewOf(resources({ processes: [process({ command: "cargo" })] }));

    expect(view.processes[0]?.command).toBe("cargo");
    expect(view.processes[0] && "args" in view.processes[0]).toBe(false);
  });

  it("carries an empty process list through, which is loud and not a gap", () => {
    expect(pulseViewOf(resources({ processes: [] })).processes).toEqual([]);
  });
});

describe("the logs a board lists", () => {
  const OWN = { kind: "job", path: ".armada/logs/1532.jsonl", bytes: 18_204, being_written: true } as const;
  const TRANSCRIPT = { kind: "transcript", path: ".armada/transcripts/1532/01D.jsonl", bytes: 912_377 } as const;

  it("is Fleet's list in Fleet's order, each the Job's own", () => {
    const view = pulseViewOf(resources({ logs: [OWN, { ...TRANSCRIPT, being_written: false }] }));

    expect(view.logs).toEqual([
      { kind: "job", path: OWN.path, owner: null, bytes: 18_204, writing: true },
      { kind: "transcript", path: TRANSCRIPT.path, owner: null, bytes: 912_377, writing: false },
    ]);
  });

  it("says nothing is writing where lsof did not answer, since no writer was seen", () => {
    expect(pulseViewOf(resources({ logs: [TRANSCRIPT] })).logs[0]?.writing).toBe(false);
  });

  it("gives no size where Fleet could not stat the file, never zero", () => {
    const view = pulseViewOf(resources({ logs: [{ kind: "brief", path: ".armada/briefs/1532/plan.1.a1.md" }] }));

    expect(view.logs[0] && "bytes" in view.logs[0]).toBe(false);
  });

  it("is empty where Fleet lists no file, whenever the Job last wrote", () => {
    expect(pulseViewOf(resources({ wrote_last_at: "2026-09-22T10:29:00Z" })).logs).toEqual([]);
  });
});

describe("when the reading was taken", () => {
  it("is carried, because a panel drawing the figures without it lies", () => {
    expect(pulseViewOf(resources()).read_at).toBe("2026-09-22T10:30:00Z");
  });

  it("carries Fleet's reading of its recorded Drone unchanged", () => {
    expect(pulseViewOf(resources({ held: "gone" })).held).toBe("gone");
  });
});
