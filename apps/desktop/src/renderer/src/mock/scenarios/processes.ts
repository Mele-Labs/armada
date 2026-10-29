// A Job's Processes section in the Artifacts layer: a running walk server, a running dev Fleet and a stopped watcher. Over the
// `one-session-view` scenario, with its Session "Queue session for later". The walk `processes` plays it.

import { seedProcesses } from "../../session-view/processes";
import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const LIVE = "one-view-5";

export const s268Processes: Scenario = {
  ...base,
  name: "processes",
  says: "A Session selected on the Cockpit, its Artifacts layer listing three processes it started: two running, one stopped",
  draft: {
    ...base.draft,
    sessions: (control) => {
      seedProcesses(LIVE, [
        { pid: 48211, command: "vite --port 5501", port: 5501, state: "running" },
        { pid: 48390, command: "armada fleet --dev", state: "running" },
        { pid: 47802, command: "vitest watch", state: "stopped" },
      ]);
      return base.draft!.sessions!(control);
    },
  },
};
