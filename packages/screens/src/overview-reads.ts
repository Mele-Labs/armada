// Overview's read, as the app holds it — `GET /health`. No React, so main imports this shape.

import type { FleetHealth, Outcome } from "@armada/protocol";

/** `GET /health`, in `reads.ts`'s four states. */
export type HealthRead =
  | { state: "none" }
  | { state: "reading" }
  | { state: "read"; health: FleetHealth }
  | { state: "failed"; outcome: Outcome };
