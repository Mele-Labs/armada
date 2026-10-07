// The window the `originsAndPanelFields` walk plays over: a request typed in
// the composer over a Fleet whose proposer fills its Job in, with a Job at its
// approval gate on the same Board whose criteria came from an issue that has
// moved since and from the request. One window, so one walk shows the settling
// caret, the origin marks, and the approval panel's request, tiers and cap.

import type { Scenario } from "@armada/bridge-api";
import type { FleetLimits } from "@armada/protocol";

/**
 * `GET /limits` as Fleet serves it: what is in force, and what shipped. The
 * panel's cap reads the machine's own cap off `concurrency`, so a scenario
 * with no limits read would draw the cap with no machine figure beside it.
 */
const LIMITS: FleetLimits = {
  concurrency: 4,
  memory_spare_percent: 15,
  disk_floor_gib: 10,
  checks_at_once: 4,
  shipped: { concurrency: 2, memory_spare_percent: 15, disk_floor_gib: 10, checks_at_once: 4 },
};

/** `filling` — `fillingIn` over a composer whose Board also holds the Job at its gate — renamed. */
export function originsAndPanel<S extends { limits: FleetLimits | null }, A, D>(filling: Scenario<S, A, D>): Scenario<S, A, D> {
  return {
    ...filling,
    name: "origins-and-panel-fields",
    says: "A request filling in beside a Job at its gate: origin marks, and the panel's request, tiers and cap",
    state: { ...filling.state, limits: LIMITS },
  };
}
