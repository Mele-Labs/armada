// The Cockpit after a cap brought Autopilot down while the person was away: its switch is off, and the setup says which cap
// did it (`slices/autopilot.ts` holds the paused one). The walk `autopilot-why` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";

import { connected } from "../moment";
import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

export const s210AutopilotWhy: Scenario = {
  name: "autopilot-why",
  says: "The Cockpit with Autopilot brought down by a cap",
  state: { ...connected([], [repository()]), repository: repository().root },
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, []) },
};
