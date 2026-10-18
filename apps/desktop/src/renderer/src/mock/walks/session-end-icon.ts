// End session in the More menu of a Session in the done state: the act wears `power-off` and the Done mark stays `power`, so
// the two no longer stand identical. At 1512 by 949 with the ledger open, where the breakdown opens
// leftward inside the panel. The Session end band's own End session is put away first, so the head's is the one pointed at.
// Over the `session-head` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const head = region("Session s-c9e3a5f1");
const sessions = region("Work", { exact: true });

const sessionEndIcon = walk(
  "session-head",
  [
    toSessions,
    { press: inside(sessions, role("button", /Release notes draft/)), say: "A Session that said it is done" },
    { press: inside(head, button("Nothing left to do")), say: "Its Session end band opens" },
    { press: inside(head, button("Keep going")), say: "Keep going puts the band away, and the state stays Done" },
    { look: inside(head, role("img", /^Done/)), say: "The Done mark stands first" },
    { hover: inside(head, role("button", "Context")), say: "The breakdown opens leftward, inside the panel" },
    { press: inside(head, button("More")), say: "The head's More menu" },
    { hover: role("menuitem", /^End session/), say: "End session" },
  ],
  { width: 1512, height: 949 },
);

export { sessionEndIcon as "session-end-icon" };
