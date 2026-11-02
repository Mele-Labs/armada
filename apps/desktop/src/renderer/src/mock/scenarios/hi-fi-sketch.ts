// A Session whose agent draws one card as hi-fi parts with `show_sketch`: HTML in sealed frames, over the stylesheets of the
// repository. Over the `sessions` scenario with one more Session open. The walk `hi-fi-sketch` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { HIFIER } from "../sessions/script";
import { s200Sessions } from "./sessions";

const hifier: Session = {
  id: HIFIER,
  title: "One card sketches",
  turn: { state: "idle" },
  rows: [{ id: "h1", at: "14:44:10", kind: "message", from: { kind: "agent" }, text: "Ask me to draw a screen." }],
  attachments: [],
};

export const s343HiFiSketch: Scenario = {
  ...s200Sessions,
  name: "hi-fi-sketch",
  says: "A Session whose agent draws one card as hi-fi parts in the repository's own stylesheets",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [hifier]) },
};
