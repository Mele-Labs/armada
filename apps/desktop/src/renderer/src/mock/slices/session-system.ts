// A Job's system, answered as Fleet would: a scenario seeds what a Job has running (`seedSystem`), the read returns it
// and stopping a process takes it out of the listing, as it is gone from Fleet's next reading.

import type { SessionSystem } from "@armada/protocol";

import type { SessionSystemApi } from "../../../../shared/api/session-system";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

const held = new Map<string, SessionSystem>();

/** Sets what a Job's system is. */
export function seedSystem(system: SessionSystem): void {
  held.set(system.session_id, system);
}

export const sessionSystem: Slice<SessionSystemApi, Record<string, never>> = {
  name: "sessionSystem",
  state: {},
  api: () => ({
    getSessionSystem: async (sessionId) => {
      const own = held.get(sessionId);
      return own === undefined ? { ok: false, outcome: unanswered(`/sessions/system?session_id=${sessionId}`) } : { ok: true, value: own };
    },
    stopProcess: async (sessionId, target) => {
      const own = held.get(sessionId);
      const found = own?.processes.find((one) => ("pid" in target ? one.pid === target.pid : one.port === target.port));
      if (own === undefined || found === undefined) {
        return {
          ok: false,
          outcome: { ok: false, why: "refused", error: { code: "fleet.process_not_the_jobs", message: "That is not a process this job started, so it is left running.", run_id: "mock", fields: {}, chain: [] } },
        };
      }
      held.set(sessionId, { ...own, processes: own.processes.filter((one) => one.pid !== found.pid) });
      return { ok: true, value: { pid: found.pid, command: found.command } };
    },
  }),
};
