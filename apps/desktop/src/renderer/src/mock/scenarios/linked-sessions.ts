// A Session that split its plan into linked Sessions it follows. Each group handed off carries its Session's row on the Plan, and the same
// Sessions stand in the parent's Ledger under Linked and under it in Work. The walk `linkedSessions` plays it. Built in `../sessions/linked.ts`.

import { linkedSessions } from "../sessions/linked";

export const s270LinkedSessions = linkedSessions("linked-sessions", "A Session and the Sessions its plan groups were handed to, drawn on the Plan and in its Ledger");
