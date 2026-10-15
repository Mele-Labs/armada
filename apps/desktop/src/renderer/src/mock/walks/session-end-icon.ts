// End session beside the Done mark on a Session in the done state: the act wears `power-off` and the mark stays `power`, so
// the two no longer stand identical. At 1512 by 949 with the ledger open, where the breakdown and the fresh card open
// leftward inside the panel. The done callout's own End session is put away first, so the head's is the one pointed at.
// Over the `session-head` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const head = region("Session s-c9e3a5f1");
const sessions = region("Sessions");

const sessionEndIcon = walk(
  "session-head",
  [
    toSessions,
    { press: inside(sessions, role("button", /Release notes draft/)), say: "A Session that said it is done" },
    { press: inside(head, button("Keep going")), say: "Keep going puts the callout away, and the state stays Done" },
    { hover: inside(head, button("End session")), say: "End session" },
    { look: inside(head, role("img", "Done")), say: "The Done mark stands last" },
    { hover: inside(head, role("button", "Context")), say: "The breakdown opens leftward, inside the panel" },
    { hover: inside(head, button("Fresh start")), say: "So does the fresh start" },
  ],
  { width: 1512, height: 949 },
);

export { sessionEndIcon as "session-end-icon" };
