// The rail mark on Workflows while Fleet has left a file out: a file that cannot run is not something to find by opening the surface.

import type { HealthRead } from "@armada/screens/src/overview-reads";

export function workflowsWarned(health: HealthRead): Readonly<Record<string, string>> {
  return health.state === "read" && health.health.workflows_left_out === true ? { workflows: "A workflow file cannot run" } : {};
}
