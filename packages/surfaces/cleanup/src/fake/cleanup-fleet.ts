// Cleanup's grid over the pool of `slots-fleet.ts`: the pool with a closed bay, two sessions' bays
// and two hosted Sessions' bays.

import type { WorktreesHeld } from "@armada/protocol";

import { slot as bay, slotsHeld } from "@armada/screens/src/fixtures/slots";

/** `GET /worktrees`: the pool of `slotsHeld` with a closed bay and bays held by sessions. */
export function gridHeld(now: number): WorktreesHeld {
  const ago = (minutes: number) => new Date(now - minutes * 60_000).toISOString();
  const base = slotsHeld(now);
  const mapped = (base.slots ?? []).map((one) => {
    if (one.slot === 3) return { ...one, closed: true };
    // An agent session whose checkout holds files nobody committed, and a clean one.
    if (one.slot === 2) {
      return {
        ...one,
        held: { state: "session" as const, holder: "nvim (pid 44698)" },
        stranded: { uncommitted: ["packages/screens/src/SlotPools.tsx", "notes/release.md"], commits: [], unpushed: 0 },
      };
    }
    if (one.slot === 6) {
      return {
        ...one,
        held: { state: "session" as const, holder: "zsh (pid 4120)" },
        branch: "fleet/slot-lease-record",
        since: ago(42),
        warm: true,
        behind: 3,
      };
    }
    return one;
  });
  // Two hosted Sessions in a bay each: one between turns, which Release frees, and one mid-turn, which Fleet refuses.
  const slots = [
    ...mapped,
    bay(9, {
      held: { state: "hosted_session", session_id: "s-theme-audit", session_title: "Theme token audit" },
      branch: "fleet/theme-token-audit",
      since: ago(18),
      warm: true,
      behind: 1,
    }),
    bay(10, {
      held: { state: "hosted_session", session_id: "s-pr-panel", session_title: "Pull Request Panel Design" },
      branch: "fleet/pull-request-panel",
      since: ago(7),
      warm: true,
      behind: 0,
    }),
  ];
  return { slots };
}

