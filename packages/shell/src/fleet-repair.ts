// Which act FleetNotice offers, and why: the one thing to press for the Fleet beside this Bridge.
//
// **Down is Start Fleet. A mismatch is a stale side, and the side decides the act**: a stale Fleet is
// restarted onto `builds/current`, a stale Bridge is reopened onto it. The kept build each side runs from is
// a fact in hand (Fleet's runtime file, this Bridge's own app folder), so the choice is read, never guessed.
// Where neither side is from a kept build, or both are stale, or the install is unknown, it is Restart
// Fleet, which is what a mismatch offered before.

import type { KeptBuilds } from "@armada/protocol";

export type Repair = "start" | "restart" | "reopen";

/** The act for a mismatch, from Fleet's build (none from a tree) and the builds this Bridge reads. */
export function staleSide(fleetBuild: string | null, builds: KeptBuilds): Repair {
  const { bridge, current } = builds;
  if (current === null) return "restart";
  if (fleetBuild !== null && fleetBuild !== current) return "restart";
  if (bridge !== null && bridge !== current) return "reopen";
  return "restart";
}

/** The act for the notice carrying `code`, or none where there is nothing to press. `builds` is null until read. */
export function repairOf(
  code: string,
  fleetBuild: string | null,
  builds: KeptBuilds | null,
): Repair | null {
  if (code === "bridge.fleet.not_running") return "start";
  if (code === "bridge.fleet.protocol_mismatch") {
    return builds === null ? null : staleSide(fleetBuild, builds);
  }
  return null;
}
