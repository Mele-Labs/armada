// The generated registries a draw borrows from: group state's glyphs are a step's, and a handed-in task reads as done.

import { GROUP_STATE, STEP_STATE, TASK_STATE } from "@armada/components/src/generated/vocabulary";
import { describe, expect, it } from "vitest";

describe("the registry rows a plan draws", () => {
  it("holds all eight group states, glyphs and all", () => {
    const states = ["pending", "running", "joining", "checking", "passed", "failed", "retrying", "landed"];
    for (const state of states) {
      expect(GROUP_STATE[state], state).toBeDefined();
      expect(GROUP_STATE[state]?.icon, state).not.toBeNull();
    }
  });

  // The glyphs are the step's one level down, which is what `[conventions.step_activity_borrowing]`
  // sanctions: never a mark minted for a group alone.
  it("borrows every group glyph from a step or a Check, never a new one", () => {
    const borrowed = new Set(
      ["not_started", "running", "advanced", "retrying"].flatMap((state) => {
        const icon = STEP_STATE[state]?.icon;
        return icon === undefined || icon === null ? [] : [icon];
      }),
    );
    // `x` is the roster's failed mark, a task's.
    const failed = TASK_STATE["failed"]?.icon;
    if (failed !== undefined && failed !== null) borrowed.add(failed);

    for (const state of ["pending", "running", "passed", "failed", "retrying", "landed"]) {
      expect(borrowed, state).toContain(GROUP_STATE[state]?.icon);
    }
  });

  it("has a mark for every task state", () => {
    for (const state of ["open", "working", "handed_in", "done", "failed", "dropped"]) {
      expect(TASK_STATE[state]?.icon, state).toBeTruthy();
    }
  });

  // A handed-in task reads as done (owner, 17 Oct 2026).
  it("gives a handed-in task done's hue and glyph", () => {
    expect(TASK_STATE.handed_in?.statusToken).toBe(TASK_STATE.done?.statusToken);
    expect(TASK_STATE.handed_in?.icon).toBe(TASK_STATE.done?.icon);
  });
});
