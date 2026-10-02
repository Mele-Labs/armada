// What more than one of `DroneTurns`' story files draws from.
import type { DroneTurn } from "./DroneTurns";

export const NOTHING_YET = "This job has no turns. It was never dispatched, so no drone has written one.";

/** A run of the Drone thinking, as the wire spells it. */
export function thinking(from: number, rows: number, at: string): DroneTurn[] {
  return Array.from({ length: rows }, (_, n) => ({
    id: String(from + n),
    at,
    who: "drone",
    kind: "unrecognised",
    subject: n % 4 === 3 ? "a turn with nothing in it Armada names" : "system/thinking_tokens",
    quiet: true,
  }));
}
