// What Fleet reported of a build, as the panel is handed it.

import { describe, expect, it } from "vitest";

import type { FleetBuildReport } from "@armada/protocol";
import { fleetBuildOf } from "./fleet-build";

const report = (over: Partial<FleetBuildReport>): FleetBuildReport => ({ on: "preview", ...over });

describe("the build as the panel is handed it", () => {
  it("carries the stage of a restart under way", () => {
    const build = fleetBuildOf(report({ restarting: "preview", stage: "reopening_bridge" }), [], () => {});
    expect(build?.working).toBe("preview");
    expect(build?.stage).toBe("reopening_bridge");
  });

  it("has no stage where Fleet reported none", () => {
    const build = fleetBuildOf(report({ restarting: "preview" }), [], () => {});
    expect(build?.working).toBe("preview");
    expect(build).not.toHaveProperty("stage");
  });
});
