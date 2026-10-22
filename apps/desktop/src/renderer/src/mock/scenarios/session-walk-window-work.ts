// `session-walk-window` with the Session on a workflow, as every Job is now, so the Cockpit draws its Flight control and the
// Artifacts layer where the Session keeps the page it showed. The walk `session-walk-window` plays it; `session-walk-window.test.tsx`
// stays on the Session without one.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { SHOWER } from "../sessions/script";
import { s200Sessions } from "./sessions";

const shower: Session = {
  id: SHOWER,
  title: "Store clock findings",
  turn: { state: "idle" },
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } },
  rows: [{ id: "w1", at: "14:36:10", kind: "message", from: { kind: "agent" }, text: "The clock reads are all in one place now." }],
  attachments: [],
};

export const s205SessionWalkWindowWork: Scenario = {
  ...s200Sessions,
  name: "session-walk-window-work",
  says: "A Session on a workflow that shows a page in a window, which the Artifacts layer and the thread keep",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [shower]) },
};
