// A pull request's chip on a Sessions row opens the card the Session ledger's row opens: the title,
// the Checks, the state and the branch. Hover or focus opens it. Over the `session-checks-weights` scenario.

import { inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Sessions");
const chip = (checks: RegExp) => inside(sessions, role("img", checks));

const card = walk("session-checks-weights", [
  toSessions,
  { hover: chip(/^Pull request #2304, Checks passed$/), say: "Hover a pull request's chip: its card opens with the title, the Checks, the state and the branch" },
  { hover: chip(/^Pull request #2301, Checks failed/), say: "A failed one names the check that failed" },
]);

export { card as "session-list-pr-card" };
