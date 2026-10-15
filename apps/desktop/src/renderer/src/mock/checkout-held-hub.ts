// A repository whose served checkout main has moved past, and what stops Fleet catching it up: four holds, one
// moment each, then main's tip reached. The hub's `checkout` is what Fleet serves (`HeldCheckout`); the card it
// raises names the one command that is safe to press, or none. `checkout-held-back.ts` plays it.
//
//   0  Two files are uncommitted and main changed one of them: an autostash could stop on a conflict.
//   1  Another branch is checked out, clean.
//   2  On main, with two commits main lacks, so the history diverged.
//   3  On main, one file uncommitted and main never touched it: the autostash pull is safe.
//   4  The checkout has main's tip: nothing is held.
//
// The first moment is the clashing one, not the safe one: a call he has opened stays opened, so the card that is
// pressed to run, and goes, comes last.

import type { HeldCheckout, MainStanding, MergeLines } from "@armada/protocol";

import type { BridgeState } from "../../../shared/bridge";

const COMMIT = "9f3c1d2e4b5a69788796a5b4c3d2e1f001122334";
const READ_AT = "2026-10-15T10:00:00Z";

const green: MainStanding = { state: "green", commit: COMMIT, read_at: READ_AT };

/** The line a moment serves: main green, nobody in the queue, and the checkout held where one is given. */
function lineWith(root: string, checkout?: HeldCheckout): MergeLines {
  return { lines: [{ root, line: [], off: [], landed: [], sent_back: [], hub: { main: green, ...(checkout === undefined ? {} : { checkout }) } }] };
}

const HELD: readonly (HeldCheckout | undefined)[] = [
  { base: "main", uncommitted: ["armada.yml", "crates/fleet/src/noticing.rs"], clashing: ["crates/fleet/src/noticing.rs"], behind: 12, ahead: 0 },
  { base: "main", on: "fix/slot-lease", behind: 40, ahead: 3 },
  { base: "main", behind: 5, ahead: 2 },
  { base: "main", uncommitted: ["armada.yml"], behind: 880, ahead: 0 },
  undefined,
];

/** The state at the first moment, and each moment after as the change a walk publishes. */
export function checkoutHeld(root: string): { first: Partial<BridgeState>; later: Partial<BridgeState>[] } {
  const at = (n: number): Partial<BridgeState> => ({ mergeLines: lineWith(root, HELD[n]) });
  return { first: at(0), later: HELD.slice(1).map((_, n) => at(n + 1)) };
}
