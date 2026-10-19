// A Session whose agent has things running in the background: two shell commands and a monitor still
// running, and two ended, one that exited 0 and one that was stopped. Over the `sessions` scenario.
// The walk `ledger-background` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { backgroundAttachments } from "../sessions/background";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const holding: Session = {
  id: "s73",
  terminal: true,
  title: "Serve the Bridge mock",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  rows: [],
  attachments: [
    { kind: "branch", name: "ledger/background", slot: 6 },
    ...backgroundAttachments,
  ],
};

export const s266LedgerBackground: Scenario = {
  ...s200Sessions,
  name: "ledger-background",
  says: "A Session with two background commands and a monitor running, and two ended: one that exited 0 and one stopped",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [holding]) },
};
