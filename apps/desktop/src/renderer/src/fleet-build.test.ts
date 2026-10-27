// What Fleet reported of a build, as the panel is handed it.

import { describe, expect, it } from "vitest";

import type { FleetBuildReport } from "@armada/protocol";
import { fleetBuildOf } from "./fleet-build";

const report = (over: Partial<FleetBuildReport>): FleetBuildReport => ({ on: "preview", ...over });

describe("the build as the panel is handed it", () => {
  it("carries the stage of a restart under way", () => {
    const build = fleetBuildOf(report({ restarting: "preview", stage: "reopening_bridge" }), () => {});
    expect(build?.working).toBe("preview");
    expect(build?.stage).toBe("reopening_bridge");
  });

  it("has no stage where Fleet reported none", () => {
    const build = fleetBuildOf(report({ restarting: "preview" }), () => {});
    expect(build?.working).toBe("preview");
    expect(build).not.toHaveProperty("stage");
  });

  it("carries the branches the preview left out, with why", () => {
    const left = [
      { branch: "armada/a", why: "conflict: apps/a.ts" },
      { branch: "armada/m", why: "breaking migration is not on main" },
    ];
    expect(fleetBuildOf(report({ left_out: left }), () => {})?.leftOut).toEqual(left);
  });

  it("has no left-out list where Fleet reported none", () => {
    expect(fleetBuildOf(report({}), () => {})).not.toHaveProperty("leftOut");
  });
});
