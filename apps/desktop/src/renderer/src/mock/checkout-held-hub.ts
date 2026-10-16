// A repository whose served checkout main has moved past, and what stops Fleet catching it up: four holds, one moment each.
// The hub's `checkout` is what Fleet serves (`HeldCheckout`); the light it raises on the merge queue names the one command that
// is safe to press, or none. `checkout-held-back.ts` plays it over the flight deck's ship.
//
//   0  On main, one file uncommitted and main never touched it: the autostash pull is safe. Run, and the checkout is level.
//   1  Two files are uncommitted and main changed one of them: an autostash could stop on a conflict.
//   2  Another branch is checked out, clean.
//   3  On main, with two commits main lacks, so the history diverged.
//
// The first `later` moment is the checkout level, so a light a Run has not yet taken away is gone before the next hold; the card of
// the holds after it stays open and changes in place.

import type { HeldCheckout, MergeLines } from "@armada/protocol";

import type { BridgeState } from "../../../shared/bridge";

const HELD: readonly HeldCheckout[] = [
  { base: "main", uncommitted: ["armada.yml"], behind: 880, ahead: 0 },
  { base: "main", uncommitted: ["armada.yml", "crates/fleet/src/noticing.rs"], clashing: ["crates/fleet/src/noticing.rs"], behind: 12, ahead: 0 },
  { base: "main", on: "fix/slot-lease", behind: 40, ahead: 3 },
  { base: "main", behind: 5, ahead: 2 },
];

/** The lines with one repository's hub carrying the checkout, or none. */
function withCheckout(lines: MergeLines, root: string, checkout?: HeldCheckout): MergeLines {
  return {
    lines: lines.lines.map((line) => {
      if (line.root !== root) return line;
      const { checkout: _held, ...hub } = line.hub ?? {};
      return { ...line, hub: { ...hub, ...(checkout === undefined ? {} : { checkout }) } };
    }),
  };
}

/** The state at the first hold, then level and each later hold as the changes a walk publishes, and the checkout level as the change a Run leaves. */
export function checkoutHeld(
  lines: MergeLines,
  root: string,
): { first: Partial<BridgeState>; later: Partial<BridgeState>[]; level: Partial<BridgeState>; safe: HeldCheckout } {
  const at = (checkout?: HeldCheckout): Partial<BridgeState> => ({ mergeLines: withCheckout(lines, root, checkout) });
  return { first: at(HELD[0]), later: [at(), ...HELD.slice(1).map(at)], level: at(), safe: HELD[0]! };
}
