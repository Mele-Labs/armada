// Which plan task a moment or a path belongs to. #1185.
import { describe, expect, it } from "vitest";

import type { PlanTask } from "@armada/protocol";

import { declaredBy, taskAt } from "./narration";

function at(seconds: number): string {
  return new Date(Date.parse("2026-09-15T08:20:00Z") + seconds * 1000).toISOString();
}

describe("the task a declared path names", () => {
  const TREE = "~/Development/armada/.armada/worktrees/01K5/";

  it("matches at a path segment and not on a shared spelling", () => {
    const tasks: PlanTask[] = [{ id: "T1", title: "a", state: "open", scope: ["crates/ipc"] }];
    expect(declaredBy(`${TREE}crates/ipc/operations.toml`, tasks)).toBe("T1");
    expect(declaredBy(`${TREE}crates/ipc-extra/x.rs`, tasks)).toBeUndefined();
  });

  it("reads a declared directory as holding the files under it", () => {
    const tasks: PlanTask[] = [{ id: "T1", title: "a", state: "open", scope: ["crates/ipc/"] }];
    expect(declaredBy(`${TREE}crates/ipc/src/lib.rs`, tasks)).toBe("T1");
  });

  it("gives the file to the exact declaration over the directory holding it", () => {
    const tasks: PlanTask[] = [
      { id: "T1", title: "a", state: "open", scope: ["crates/ipc"] },
      { id: "T2", title: "b", state: "open", scope: ["crates/ipc/operations.toml"] },
    ];
    expect(declaredBy(`${TREE}crates/ipc/operations.toml`, tasks)).toBe("T2");
  });

  it("gives a tie to plan order, and says nothing where no task named the path", () => {
    const same: PlanTask[] = [
      { id: "T1", title: "a", state: "open", scope: ["crates/ipc/operations.toml"] },
      { id: "T2", title: "b", state: "open", scope: ["crates/ipc/operations.toml"] },
    ];
    expect(declaredBy(`${TREE}crates/ipc/operations.toml`, same)).toBe("T1");
    expect(declaredBy(`${TREE}crates/store/src/lib.rs`, same)).toBeUndefined();
  });

  it("says nothing for a task that declared nothing", () => {
    const bare: PlanTask[] = [{ id: "T1", title: "a", state: "open" }];
    expect(declaredBy(`${TREE}crates/ipc/x.rs`, bare)).toBeUndefined();
  });
});

describe("which task a moment belongs to", () => {
  it("is the one entered last where two windows overlap", () => {
    const overlapping: PlanTask[] = [
      { id: "T1", title: "a", state: "working", working_windows: [{ entered: at(0) }] },
      { id: "T2", title: "b", state: "working", working_windows: [{ entered: at(5) }] },
    ];
    expect(taskAt(at(3), overlapping)).toBe("T1");
    expect(taskAt(at(6), overlapping)).toBe("T2");
  });

  it("is none at the instant a window closes", () => {
    const closed: PlanTask[] = [
      { id: "T1", title: "a", state: "done", working_windows: [{ entered: at(0), left: at(5) }] },
    ];
    expect(taskAt(at(5), closed)).toBeUndefined();
  });
});
