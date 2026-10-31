// A Job's System layer: its processes, CPU, memory and disk, the two worktrees it holds and its logs. Over the
// `one-session-view` scenario, with its Session "Queue session for later". The walk `processes` plays it.

import type { WorktreeSlot } from "@armada/protocol";

import { seedReadings } from "../../session-view/system";
import { seedSystem } from "../slices/session-system";
import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const LIVE = "one-view-5";
const KB = 1024;
const GB = KB * KB * KB;

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
      seedSystem({
        session_id: LIVE,
        processes: [
          { pid: 48211, command: "vite --port 5501", port: 5501, cpu_percent: 12, memory_bytes: 310 * KB * KB, io_bytes_per_s: 48 * KB, state: "running" },
          { pid: 48390, command: "armada fleet --dev", cpu_percent: 21, memory_bytes: 1.2 * GB, io_bytes_per_s: 2.4 * KB * KB, state: "running" },
          { pid: 47802, command: "vitest watch", state: "stopped" },
          ...[1, 2, 3, 4, 5, 6, 7].map((n) => ({ pid: 47900 + n, command: `node worker-${n}.js`, cpu_percent: n * 2, memory_bytes: (80 + n * 11) * KB * KB, io_bytes_per_s: n * 9 * KB, state: "running" as const })),
        ],
        cpu_percent: 38,
        memory_bytes: 1.8 * GB,
        memory_of_bytes: 8 * GB,
        disk_bytes: 6.4 * GB,
        disk_of_bytes: 20 * GB,
        worktrees: [],
        logs: [
          { name: "session.log", path: `/mock/keepers/${LIVE}.log`, bytes: 96 * KB },
          { name: `${LIVE}.jsonl`, path: `/mock/transcripts/${LIVE}.jsonl`, bytes: 3.2 * KB * KB },
        ],
      }, 2500);
      const wave = (mid: number, swing: number) => Array.from({ length: 60 }, (_, i) => Math.max(0, mid + Math.sin(i / 3) * swing + Math.sin(i * 1.7) * swing * 0.4));
      seedReadings(LIVE, wave(38, 22), wave(1.8, 0.3));
      return base.draft!.sessions!(control);
    },
  },
};
