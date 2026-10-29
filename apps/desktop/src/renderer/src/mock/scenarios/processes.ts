// A Job's System layer: its processes, CPU, memory and disk, the two worktrees it holds and three logs. Over the
// `one-session-view` scenario, with its Session "Queue session for later". The walk `processes` plays it.

import type { WorktreeSlot } from "@armada/protocol";

import { seedSystem } from "../../session-view/system";
import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const LIVE = "one-view-5";
const KB = 1024;

export const s268Processes: Scenario = {
  ...base,
  name: "processes",
  says: "A Session selected on the Cockpit, its System layer listing its processes, CPU, memory, disk, worktrees and logs",
  held: {
    ...base.held,
    slots: [
      ...(base.held?.slots ?? []),
      ...[5, 6].map(
        (slot): WorktreeSlot => ({
          manifest_id: base.held?.slots?.[0]?.manifest_id ?? "armada",
          slot,
          path: `/Users/user/armada/.armada/slots/slot-${slot}`,
          base: "main",
          warm: true,
          behind: 0,
          held: { state: "hosted_session", session_id: LIVE, session_title: "Queue session for later" },
          branch: slot === 5 ? "armada/queue-later" : "fix/queue-ordering",
        }),
      ),
    ],
  },
  draft: {
    ...base.draft,
    sessions: (control) => {
      seedSystem(LIVE, {
        processes: [
          { pid: 48211, command: "vite --port 5501", port: 5501, state: "running" },
          { pid: 48390, command: "armada fleet --dev", state: "running" },
          { pid: 47802, command: "vitest watch", state: "stopped" },
        ],
        cpu: { used: 38, of: 100, unit: "%" },
        memory: { used: 1.8, of: 8, unit: "GB" },
        disk: { used: 6.4, of: 20, unit: "GB" },
        logs: [
          { name: "vite.log", bytes: 412 * KB },
          { name: "fleet-dev.log", bytes: 3.2 * KB * KB },
          { name: "vitest.log", bytes: 96 * KB },
        ],
      });
      return base.draft!.sessions!(control);
    },
  },
};
