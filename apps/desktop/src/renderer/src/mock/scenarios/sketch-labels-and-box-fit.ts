// A Session whose agent drew how data moves through Fleet: arrows converging on one box, and a box whose body is longer than the
// height the agent gave it. Over the `sessions` scenario with one more Session open. The walk `sketch-labels-and-box-fit` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { HUBBER } from "../sessions/script";
import { s200Sessions } from "./sessions";

const hubber: Session = {
  id: HUBBER,
  title: "How data moves",
  turn: { state: "idle" },
  rows: [{ id: "h1", at: "14:44:10", kind: "message", from: { kind: "agent" }, text: "Ask me for a diagram of how Armada moves data." }],
  attachments: [],
};

export const s247SketchLabelsAndBoxFit: Scenario = {
  ...s200Sessions,
  name: "sketch-labels-and-box-fit",
  says: "A Session whose agent draws the data flow: converging labelled arrows and a long body",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [hubber]) },
};
