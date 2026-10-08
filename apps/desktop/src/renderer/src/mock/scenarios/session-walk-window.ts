// A Session that shows a page: spoken to, it opens a window on a page of its own accord, and the
// ledger and the thread keep it so it can be opened again. Over the `sessions` scenario with one
// more Session open beside its own. The walk `session-walk-window` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { SHOWER } from "../sessions/script";
import { s200Sessions } from "./sessions";

const shower: Session = {
  id: SHOWER,
  title: "Store clock findings",
  turn: { state: "idle" },
  lastTurn: "14:36",
  rows: [{ id: "w1", at: "14:36:10", kind: "message", from: { kind: "agent" }, text: "The clock reads are all in one place now." }],
  attachments: [],
};

export const s205SessionWalkWindow: Scenario = {
  ...s200Sessions,
  name: "session-walk-window",
  says: "A Session that shows a page in a window, which the ledger and the thread keep",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [shower]) },
};
