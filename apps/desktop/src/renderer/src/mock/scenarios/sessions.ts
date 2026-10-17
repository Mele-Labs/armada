// Sessions, drawn ahead of Fleet: raw conversations with a hosted agent, beside
// Helm. Cleanup is the real one over real fixtures; the Sessions themselves are draft
// data (`packages/screens/src/draft/sessions.ts`) the scenario's script plays moment by
// moment. The walk `sessions` plays it.

import type { WorktreeSlot, WorktreesHeld } from "@armada/protocol";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import type { Session } from "@armada/screens/src/draft/sessions";
import { sessionsStore } from "../sessions/script";
import type { BayControl } from "../sessions/script";

const AT = "/Users/user/armada/.armada/slots";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const bay = (n: number, branch: string, held: WorktreeSlot["held"], minutes: number): WorktreeSlot => ({
  manifest_id: "armada",
  slot: n,
  path: `${AT}/slot-${n}`,
  base: "main",
  warm: true,
  behind: 0,
  since: ago(minutes),
  branch,
  held,
});

const held = (): WorktreesHeld => ({
  slots: [
    { manifest_id: "armada", slot: 2, path: `${AT}/slot-2`, base: "main", warm: true, behind: 0, held: { state: "free" } },
    bay(3, "fix/flaky-store", { state: "session", holder: "Session s7" }, 6),
    bay(5, "rel/notes-script", { state: "session", holder: "Session s2" }, 40),
    bay(7, "spike/store-migrations", { state: "session", holder: "Session s3" }, 120),
  ],
});

const base = holding("sessions", "Sessions beside Helm: a blank one, its first write, and what it accumulates");

export const s200Sessions: Scenario = {
  ...base,
  held: held(),
  draft: { sessions: sessionsOf },
};

/** The walk's Sessions store, with `more` Sessions open beside the usual ones. */
function sessionsOf(_board: BayControl, more: readonly Session[] = []) {
  return sessionsStore(more);
}
