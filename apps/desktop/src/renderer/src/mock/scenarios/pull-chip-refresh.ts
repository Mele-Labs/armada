// A Job holding a pull request in the merge line with its checks running, and a teammate's beside it that no Job holds.
// Refresh on either chip reads the forge now; #2290 reads failed. The walks `pull-chip-refresh` and `pull-chip-open-job` play it.

import type { Scenario } from "../moment";
import { configFleet, configScenario } from "../sessions/pull-chip-refresh";

export const pullChipRefresh: Scenario = {
  ...configScenario(configFleet()),
  name: "pull-chip-refresh",
  says: "A Job's pull request in the merge line with its checks running, which Refresh reads failed, and a teammate's no Job holds",
  // A Fleet of its own each time, so a walk told twice starts from checks running.
  behaves: (handle) => configScenario(configFleet()).behaves!(handle),
};
