// A Session that shows three walks in one window: each take is a page that plays a walk, and the
// window's bar draws that walk's controls. Over the `sessions` scenario with one more Session beside
// its own. The walk `walk-in-the-window` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { WALKER } from "../sessions/script";
import { s200Sessions } from "./sessions";

const walker: Session = {
  id: WALKER,
  title: "Walks in a window",
  turn: { state: "idle" },
  rows: [{ id: "w1", at: "14:41:02", kind: "message", from: { kind: "agent" }, text: "Three walks are built." }],
  attachments: [],
};

export const s206WalkInTheWindow: Scenario = {
  ...s200Sessions,
  name: "walk-in-the-window",
  says: "A Session that shows three walks in one window, their controls on its bar",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [walker]) },
};
