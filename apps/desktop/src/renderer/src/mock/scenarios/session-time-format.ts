// A Session whose thread holds stamps from the last hour: a lease, the agent's words, the person's
// own, an Approve and a changed value. Pressing one turns every stamp in the thread between the clock
// and the age; Settings → Appearance → Time sets the clock's format. Over the `sessions` scenario.
// The walk `time-format` plays it.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

/** A row dated `minutes` ago, as the wire dates its rows. */
const dated = (minutes: number, row: DistributiveOmit<SessionRow, "at">): SessionRow => {
  return { ...row, at: new Date(Date.now() - minutes * 60_000).toISOString() } as SessionRow;
};

const told: Session = {
  id: "s41",
  title: "Retry backoff",
  turn: { state: "idle" },
  lastTurnAt: new Date(Date.now() - 2 * 60_000).toISOString(),
  rows: [
    dated(47, { id: "t1", kind: "lease", slot: 3, branch: "fix/retry-backoff" }),
    dated(31, { id: "t2", kind: "message", from: { kind: "agent" }, text: "The retry waits one second, then two, then four. Capped at thirty." }),
    dated(9, { id: "t3", kind: "message", from: { kind: "you" }, text: "Cap it at ten." }),
    dated(6, { id: "t4", kind: "message", from: { kind: "you" }, text: "Approved: http://localhost:5188/?walk=retry-backoff" }),
    dated(4, { id: "t5", kind: "change", what: "Retry cap", from: "30 s", to: "10 s" }),
    dated(2, { id: "t6", kind: "message", from: { kind: "agent" }, text: "The cap is ten seconds. The test waits on the clock it is given." }),
  ],
  attachments: [{ kind: "slot", slot: 3 }, { kind: "branch", name: "fix/retry-backoff", slot: 3 }],
};

export const s310SessionTimeFormat: Scenario = {
  ...s200Sessions,
  name: "session-time-format",
  says: "A Session's thread with stamps from the last hour, and Settings → Time for the format they are drawn in",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [told]) },
};
