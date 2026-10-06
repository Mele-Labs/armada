// Which Drone owns a held command: the wire names the step, so only a step with
// one Drone at work can say.

import { describe, expect, it } from "vitest";

import type { JobDetail } from "@armada/protocol";
import type { DroneView } from "./draft/drone";
import { holderOf } from "./drone-held";
import { runningWaitingOnACommand } from "./fixtures/build/index";

function held(): JobDetail {
  const fixture = runningWaitingOnACommand();
  if (fixture.watched.state !== "read") throw new Error("fixture is not read");
  return fixture.watched.detail;
}

const drone = (id: string, over: Partial<DroneView> = {}): DroneView => ({
  id,
  step: "fix",
  state: "running",
  ...over,
});

describe("the Drone a held command is on", () => {
  it("is the one Drone at work on the step the wire names", () => {
    expect(holderOf(held(), [drone("a"), drone("other", { step: "repro", state: "done" })])?.id).toBe("a");
  });

  it("is nobody where several work the step at once, because the wire cannot say which", () => {
    expect(holderOf(held(), [drone("a", { task: "T1" }), drone("b", { task: "T2" })])).toBeUndefined();
  });

  it("is not a Drone at rest, which is not the one inside a call", () => {
    expect(holderOf(held(), [drone("a", { at_rest_since: "2026-09-10T14:00:00Z" })])).toBeUndefined();
  });

  it("is nobody where nothing is held", () => {
    const whole = { ...held(), command_waiting: undefined };
    expect(holderOf(whole, [drone("a")])).toBeUndefined();
  });
});
