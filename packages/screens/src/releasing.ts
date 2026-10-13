// What a refused Release says. Its own file because Cleanup's panel and the
// Cockpit's Bays row both send the act and must word its refusal alike.

import type { Outcome } from "@armada/protocol";
import { said } from "./copy";

/**
 * Fleet's own words behind what did not happen. `working` is the one refusal
 * with a Session to open: the Session holding the slot is mid-turn,
 * `fleet.slot_session_working`.
 */
export function releaseRefusal(outcome: Outcome): { said: string; working: boolean } {
  const refused = !outcome.ok && outcome.why === "refused";
  const why = refused ? outcome.error.message : said(outcome);
  return { said: `Slot not released: ${why}`, working: refused && outcome.error.code === "fleet.slot_session_working" };
}
