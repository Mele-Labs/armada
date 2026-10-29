// What a Job costs the machine, for its System layer: read for one Job, and one process of it stopped. A slice
// imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Outcome, ProcessStopped, SessionSystem } from "@armada/protocol";

/** What reading a Job's system came to: Fleet's readings, or the refusal to word. */
export type SessionSystemRead = { ok: true; value: SessionSystem } | { ok: false; outcome: Outcome };

/** What stopping a process came to: the process ended, or Fleet's refusal. */
export type ProcessStopRead = { ok: true; value: ProcessStopped } | { ok: false; outcome: Outcome };

/** The process to stop, by its pid or by the port it listens on. */
export type ProcessTarget = { pid: number } | { port: number };

export type SessionSystemApi = {
  /** `GET /sessions/system`: the processes, load, worktrees and logs of one Job. */
  getSessionSystem: (sessionId: string) => Promise<SessionSystemRead>;
  /** `POST /sessions/stop_process`: ends a process the Job started. Fleet refuses any other. */
  stopProcess: (sessionId: string, target: ProcessTarget) => Promise<ProcessStopRead>;
};

export const SESSION_SYSTEM_CHANNELS = {
  getSessionSystem: "bridge:get-session-system",
  stopProcess: "bridge:stop-process",
} as const;
