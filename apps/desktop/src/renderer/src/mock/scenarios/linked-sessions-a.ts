// A Session that split its plan into linked Sessions it follows, first take: each handed off group on the Plan carries its Session's row.
// The Sessions stand under their parent in Work. The walk `linkedSessionsA` plays it. Built in `../sessions/linked.ts`.

import { linkedSessions } from "../sessions/linked";

export const s270LinkedSessionsA = linkedSessions("linked-sessions-a", "plan", "A Session and the Sessions its plan groups were handed to, drawn on the Plan");
