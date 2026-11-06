// Revert to the build before, asked from the menu or the palette: Cancel does nothing, Revert asks Fleet, a Fleet that is not
// there is bypassed by the build before's own undo when that file exists, and a refusal or a missing build is said in a dialog.

import { describe, expect, it, vi } from "vitest";

import type { FleetBuildReport, Outcome } from "@armada/protocol";
import { NO_BUILD_BEFORE, revertAsked, type RevertAsking, type RevertBox } from "./revert-asked";

const WITH_PREVIOUS: FleetBuildReport = {
  on: "main",
  previous: {
    built_at: "2026-11-05T22:00:00Z",
    commit: "b1c2d3e4f5".padEnd(40, "0"),
    takes_out: [
      "Fix the board",
      "Add a chime",
    ],
    bridge_reopens: true,
  },
};
const UNDO = "/Users/p/Library/Application Support/Armada/builds/previous/armada";

function rig(
  options: {
    report?: FleetBuildReport | null;
    presses?: number[];
    outcome?: Outcome | undefined;
    exists?: boolean;
    undo?: string | null;
  } = {},
) {
  const presses = [
    ...(options.presses ?? [
      0,
    ]),
  ];
  const boxes: RevertBox[] = [];
  const revert = vi.fn(async () =>
    "outcome" in options
      ? options.outcome
      : {
          ok: true as const,
        },
  );
  const run = vi.fn();
  const asking: RevertAsking = {
    report: () => ("report" in options ? options.report! : WITH_PREVIOUS),
    revert,
    show: async (box) => {
      boxes.push(box);
      return presses.shift() ?? 0;
    },
    undo: () => ("undo" in options ? options.undo! : UNDO),
    exists: () => options.exists ?? true,
    run,
  };
  return {
    asking,
    boxes,
    revert,
    run,
  };
}

const NOT_CONNECTED: Outcome = {
  ok: false,
  why: "not_connected",
};

describe("the confirm", () => {
  it("names the build before, what it takes out and that Bridge reopens, with Cancel the default", async () => {
    const { asking, boxes } = rig({
      presses: [
        1,
      ],
    });
    await revertAsked(asking);
    expect(boxes).toHaveLength(1);
    expect(boxes[0]).toMatchObject({
      message: "Revert to the build before?",
      buttons: [
        "Revert",
        "Cancel",
      ],
      defaultId: 1,
      cancelId: 1,
    });
    expect(boxes[0]!.detail).toBe(
      "Built 2026-11-05T22:00:00Z, commit b1c2d3e4\nTakes out: Fix the board, Add a chime\nBridge reopens",
    );
  });

  it("leaves out Takes out and Bridge reopens when there are none", async () => {
    const { asking, boxes } = rig({
      report: {
        on: "main",
        previous: {
          built_at: "t",
          commit: "c".repeat(40),
        },
      },
      presses: [
        1,
      ],
    });
    await revertAsked(asking);
    expect(boxes[0]!.detail).toBe("Built t, commit cccccccc");
  });

  it("does nothing on Cancel", async () => {
    const { asking, revert, run, boxes } = rig({
      presses: [
        1,
      ],
    });
    await revertAsked(asking);
    expect(revert).not.toHaveBeenCalled();
    expect(run).not.toHaveBeenCalled();
    expect(boxes).toHaveLength(1);
  });
});

describe("Revert", () => {
  it("asks Fleet to revert, and says nothing more when it starts", async () => {
    const { asking, revert, run, boxes } = rig();
    await revertAsked(asking);
    expect(revert).toHaveBeenCalledTimes(1);
    expect(run).not.toHaveBeenCalled();
    expect(boxes).toHaveLength(1);
  });

  it("runs the build before's own undo, detached, when Fleet is not connected and the file exists", async () => {
    const { asking, run, boxes } = rig({
      outcome: NOT_CONNECTED,
    });
    await revertAsked(asking);
    expect(run).toHaveBeenCalledWith(UNDO, [
      "build",
      "revert",
    ]);
    expect(boxes).toHaveLength(1);
  });

  it("treats no connection at all as Fleet not connected", async () => {
    const { asking, run } = rig({
      outcome: undefined,
    });
    await revertAsked(asking);
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("runs nothing and says so when the undo is not there", async () => {
    for (const options of [
      {
        exists: false,
      },
      {
        undo: null,
      },
    ]) {
      const { asking, run, boxes } = rig({
        outcome: NOT_CONNECTED,
        ...options,
      });
      await revertAsked(asking);
      expect(run).not.toHaveBeenCalled();
      expect(boxes.at(-1)).toMatchObject({
        message: "Fleet is not connected",
      });
    }
  });

  it("shows a refusal's reason, and does not run the undo", async () => {
    const { asking, run, boxes } = rig({
      outcome: {
        ok: false,
        why: "refused",
        error: {
          code: "fleet.revert_busy",
          message: "a Drone is working",
          run_id: "r",
          fields: {},
          chain: [],
        },
      },
    });
    await revertAsked(asking);
    expect(run).not.toHaveBeenCalled();
    expect(boxes).toHaveLength(2);
    expect(boxes[1]).toMatchObject({
      message: "Revert did not start",
      detail: "a Drone is working",
    });
  });
});

describe("no build before", () => {
  it.each([
    null,
    {
      on: "main",
    } as FleetBuildReport,
  ])("says so and does nothing, for the report %j", async (report) => {
    const { asking, revert, run, boxes } = rig({
      report,
    });
    await revertAsked(asking);
    expect(boxes).toHaveLength(1);
    expect(boxes[0]!.message).toBe(NO_BUILD_BEFORE);
    expect(revert).not.toHaveBeenCalled();
    expect(run).not.toHaveBeenCalled();
  });
});
