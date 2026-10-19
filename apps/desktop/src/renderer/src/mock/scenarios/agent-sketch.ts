// A Session whose agent draws a sketch when it is spoken to: a card in the thread where it did, the sketch
// open in the panel, a second one with the same title that replaces it, and a later one read as a change to it.
// Over the `sessions` scenario with one more Session open beside its own. The walk `agent-sketch` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { SKETCHER } from "../sessions/script";
import { s200Sessions } from "./sessions";

const sketcher: Session = {
  id: SKETCHER,
  title: "Store clock sketches",
  turn: { state: "idle" },
  rows: [{ id: "k1", at: "14:44:10", kind: "message", from: { kind: "agent" }, text: "The clock reads are all in one place now." }],
  attachments: [],
};

export const s243AgentSketch: Scenario = {
  ...s200Sessions,
  name: "agent-sketch",
  says: "A Session whose agent draws a sketch, replaces it, and draws a change to it",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [sketcher]) },
};
