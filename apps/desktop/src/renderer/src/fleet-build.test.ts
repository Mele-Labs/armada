// What Fleet reported of a build, as the panel is handed it.

import { describe, expect, it, vi } from "vitest";

import type { FleetBuildReport } from "@armada/protocol";
import { builtSaid, fleetBuildOf } from "./fleet-build";

const report = (over: Partial<FleetBuildReport>): FleetBuildReport => ({
  on: "preview",
  ...over,
});

describe("the build as the panel is handed it", () => {
  it("carries the stage of a restart under way", () => {
    const build = fleetBuildOf(
      report({
        restarting: "preview",
        stage: "reopening_bridge",
      }),
      () => {},
      () => {},
    );
    expect(build?.working).toBe("preview");
    expect(build?.stage).toBe("reopening_bridge");
  });

  it("has no stage where Fleet reported none", () => {
    const build = fleetBuildOf(
      report({
        restarting: "preview",
      }),
      () => {},
      () => {},
    );
    expect(build?.working).toBe("preview");
    expect(build).not.toHaveProperty("stage");
  });

  it("carries the branches the preview left out, with why", () => {
    const left = [
      {
        branch: "armada/a",
        why: "conflict: apps/a.ts",
      },
      {
        branch: "armada/m",
        why: "breaking migration is not on main",
      },
    ];
    expect(
      fleetBuildOf(
        report({
          left_out: left,
        }),
        () => {},
        () => {},
      )?.leftOut,
    ).toEqual(left);
  });

  it("has no left-out list where Fleet reported none", () => {
    expect(
      fleetBuildOf(
        report({}),
        () => {},
        () => {},
      ),
    ).not.toHaveProperty("leftOut");
  });

  it("draws no kept builds, no Revert and no carried Jobs where Fleet reported none", () => {
    const build = fleetBuildOf(
      report({}),
      () => {},
      () => {},
    );
    for (const key of [
      "running",
      "previous",
      "reverting",
      "carried",
    ]) {
      expect(build).not.toHaveProperty(key);
    }
  });

  it("names a kept build by its local time and its commit cut to seven", () => {
    const at = new Date(2026, 10, 5, 17, 5).toISOString();
    const build = fleetBuildOf(
      report({
        running: {
          built_at: at,
          commit: "e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3",
        },
      }),
      () => {},
      () => {},
    );
    expect(build?.running).toEqual({
      built: "5 Nov, 17:05",
      commit: "e4f5a6b",
    });
  });

  it("carries the build before with what a revert takes out and brings back, as Fleet named them", () => {
    const build = fleetBuildOf(
      report({
        previous: {
          built_at: new Date(2026, 10, 5, 9, 0).toISOString(),
          commit: "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
          bridge_reopens: true,
          takes_out: [
            "Fleet core and mods",
          ],
          brings_back: [
            "armada/old-branch",
          ],
        },
      }),
      () => {},
      () => {},
    );
    expect(build?.previous).toEqual({
      built: "5 Nov, 09:00",
      commit: "a1b2c3d",
      bridgeReopens: true,
      takesOut: [
        "Fleet core and mods",
      ],
      bringsBack: [
        "armada/old-branch",
      ],
    });
  });

  it("reads the lists and the reopen a previous build leaves out as empty and no", () => {
    const build = fleetBuildOf(
      report({
        previous: {
          built_at: new Date(2026, 10, 5, 9, 0).toISOString(),
          commit: "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
        },
      }),
      () => {},
      () => {},
    );
    expect(build?.previous).toMatchObject({
      bridgeReopens: false,
      takesOut: [],
      bringsBack: [],
    });
  });

  it("carries a revert under way and the Jobs at work, and presses revert through", () => {
    const revert = vi.fn();
    const build = fleetBuildOf(
      report({
        reverting: true,
        carried: [
          "Kept builds",
        ],
      }),
      () => {},
      revert,
    );
    expect(build?.reverting).toBe(true);
    expect(build?.carried).toEqual([
      "Kept builds",
    ]);
    build?.onRevert?.();
    expect(revert).toHaveBeenCalledTimes(1);
  });

  it("keeps text that is no instant as it came", () => {
    expect(builtSaid("not a time")).toBe("not a time");
  });
});
