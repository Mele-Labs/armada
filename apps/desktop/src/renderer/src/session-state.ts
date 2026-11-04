// What state a Session is in, as Bridge reads it: `jobStateOf`, with a reply unread until he has been in the Session
// since it ended (`cockpit/viewed.ts`), and then the Session stands as it would without one.

import {
  holdsDelivery,
  jobStateOf,
  repairSaid,
  restartSaid,
  stoppedSaid,
  waitsOf,
} from "@armada/screens/src/session-state";
import type { Session } from "@armada/screens/src/draft/sessions";

import { viewedAt } from "./cockpit/viewed";

export { holdsDelivery, repairSaid, restartSaid, stoppedSaid, waitsOf };

/** Whether he has been in the Session since its turn ended on a reply. */
const readSince = (session: Session): boolean =>
  session.lastTurnAt !== undefined && viewedAt(session.id) >= Date.parse(session.lastTurnAt);

export const stateOf = (session: Session): ReturnType<typeof jobStateOf> =>
  jobStateOf(session, readSince);
