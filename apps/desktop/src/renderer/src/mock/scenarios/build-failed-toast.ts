// A restart onto the preview that fails. Fleet runs on main and reports the preview restart under way; the next read says
// it failed, with the compiler's line, and the failure toast stands with Dispatch session. A walk's `later` step publishes
// the failed report. Over the `sessions` scenario. The walk `build-failed-toast` plays it.

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const COMMIT = "8944d2fca1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";

export const s274BuildFailedToast: Scenario = {
  ...s200Sessions,
  name: "build-failed-toast",
  says: "A restart onto the preview is under way and its next read says it failed; the failure toast offers Dispatch session",
  state: { ...s200Sessions.state, fleetBuild: { on: "main", commit: COMMIT, position: { ahead: 0, behind: 2 }, restarting: "preview", stage: "building_fleet" } },
  later: [
    { fleetBuild: { on: "main", commit: COMMIT, position: { ahead: 0, behind: 2 }, failed: "error[E0432]: unresolved import crate::drones" } },
  ],
};
