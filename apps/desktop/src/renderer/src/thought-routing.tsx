// Helm routes a thought into running work. Draft, mock-only: nothing in Fleet reads words for where
// they belong yet, so the window holds a reader only where a mock mounts one. A real window holds none
// and Helm's dock asks every message as it always has.
//
// The reader says where the words belong; delivering them is the paths that already exist. A Session
// gets a message, a Job a redirect to its Drone, and a new one opens the Dispatch panel.

import { createContext, useContext, type ReactNode } from "react";
import type { JobSummary, RepositorySummary, SessionRecord } from "@armada/protocol";
import type { RouteTarget, ThoughtReading, ThoughtRouting } from "@armada/helm";

import type { BridgeState } from "../../shared/bridge";
import type { Dispatching } from "./dispatch-panel/dispatching";
import { useSessionsDraft } from "./sessions-draft";

/** Everything live a thought could go to: every repository's Sessions and Jobs, whatever Helm points at. */
export type ThoughtWorld = {
  sessions: readonly SessionRecord[];
  jobs: readonly JobSummary[];
  repositories: readonly RepositorySummary[];
};

export type ThoughtReader = (text: string, world: ThoughtWorld) => Promise<ThoughtReading>;

const Held = createContext<ThoughtReader | undefined>(undefined);

/** Hand a window its reader. The mock's own mount is the one caller. */
export function ThoughtReadingFrom({ reader, children }: { reader: ThoughtReader | undefined; children: ReactNode }) {
  return <Held.Provider value={reader}>{children}</Held.Provider>;
}

type Host = {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  dispatching: Dispatching;
  openSession: (id: string) => void;
  openJob: (jobId: string) => void;
};

/** The dock's routing, or `undefined` where no reader is held or no Sessions are served. */
export function useThoughtRouting({ state, repositories, dispatching, openSession, openJob }: Host): ThoughtRouting | undefined {
  const reader = useContext(Held);
  const draft = useSessionsDraft();
  if (reader === undefined || draft === undefined) return undefined;
  const world: ThoughtWorld = {
    sessions: state.sessions.state === "read" ? state.sessions.sessions : [],
    jobs: state.jobs,
    repositories,
  };
  return {
    read: (text) => reader(text, world),
    send: (target: RouteTarget, text) => {
      if (target.kind === "session") draft.send(target.id, { text, files: [], sketches: [], tags: [] });
      else void window.armada.redirectDrone(target.id, text);
    },
    open: (target) => (target.kind === "session" ? openSession(target.id) : openJob(target.id)),
    startNew: (kind, text) => {
      dispatching.choose(kind);
      dispatching.openTitled(text);
    },
  };
}
