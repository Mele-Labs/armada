// A Session that split its plan into linked Sessions it follows, second take: a Linked section in the parent's Ledger.
// The Sessions stand under their parent in Work. The walk `linkedSessionsB` plays it. Built in `../sessions/linked.ts`.

import { linkedSessions } from "../sessions/linked";

export const s271LinkedSessionsB = linkedSessions("linked-sessions-b", "ledger", "A Session and the Sessions its plan groups were handed to, drawn in its Ledger");
