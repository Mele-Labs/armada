// A Session that shows three takes of one page in one window: the person swaps between them on the
// bar, a note names the take it was taken on, and Approve approves the one on screen. Over the
// `sessions` scenario with one more Session beside its own. The walk `walk-variations` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { VARIER } from "../sessions/script";
import { s200Sessions } from "./sessions";

const varier: Session = {
  id: VARIER,
  title: "Board layout takes",
  turn: { state: "idle" },
  rows: [{ id: "v1", at: "14:41:02", kind: "message", from: { kind: "agent" }, text: "Three layouts for the Board are built." }],
  attachments: [],
};

export const s205WalkVariations: Scenario = {
  ...s200Sessions,
  name: "walk-variations",
  says: "A Session that shows three takes of one page, swapped on the window's bar",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [varier]) },
};
