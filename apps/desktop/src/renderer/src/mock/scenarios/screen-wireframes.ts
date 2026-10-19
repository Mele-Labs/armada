// A Session whose agent drew screens with `show_sketch`: a desktop window with a modal, a web app in a browser frame, and a
// flow of two phone screens joined by an arrow. Over the `sessions` scenario with one more Session open. The walk
// `screen-wireframes` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { WIRER } from "../sessions/script";
import { s200Sessions } from "./sessions";

const wirer: Session = {
  id: WIRER,
  title: "Screen wireframes",
  turn: { state: "idle" },
  rows: [{ id: "w1", at: "14:44:10", kind: "message", from: { kind: "agent" }, text: "Ask me to draw a screen." }],
  attachments: [],
};

export const s268ScreenWireframes: Scenario = {
  ...s200Sessions,
  name: "screen-wireframes",
  says: "A Session whose agent wireframes screens: a window with a modal, a web app, and a phone flow",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [wirer]) },
};
