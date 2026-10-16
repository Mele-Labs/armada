// A repository whose served checkout main has moved past, and what stops Fleet catching it up: four holds, one moment each.
// The hub's `checkout` is what Fleet serves (`HeldCheckout`); the light it raises on the merge queue names the one command that
// is safe to press, or none. `checkout-held-back.ts` plays it over the flight deck's ship.
//
//   0  On main, one file uncommitted and main never touched it: the autostash pull is safe. Run, and the checkout is level.
//   1  Two files are uncommitted and main changed one of them: an autostash could stop on a conflict.
//   2  On main, with two commits main lacks, so the history diverged.
//   3  A loose agent left 100 files uncommitted, three of them changed on main too: the list is long, the clashing ones scattered through it.
//
// The first `later` moment is the checkout level, so a light a Run has not yet taken away is gone before the next hold; the card of
// the holds after it stays open and changes in place.

import type { HeldCheckout, MergeLines } from "@armada/protocol";

import type { BridgeState } from "../../../shared/bridge";

/** 100 uncommitted files under crates/ and apps/, in the order git would list them, with three that main also changed placed through them. */
const LOOSE_CLASHING: string[] = ["crates/fleet/src/noticing.rs", "apps/desktop/src/renderer/src/Dashboard.tsx", "crates/fleet/src/slots/lease.rs"];
const LOOSE: string[] = (() => {
  const others = Array.from({ length: 97 }, (_, i) => {
    const n = String(i + 1).padStart(2, "0");
    return i % 2 === 0 ? `crates/fleet/src/loose/part${n}.rs` : `apps/desktop/src/renderer/src/loose/part${n}.tsx`;
  });
  const [a, b, c] = LOOSE_CLASHING as [string, string, string];
  return [...others.slice(0, 17), a, ...others.slice(17, 48), b, ...others.slice(48, 83), c, ...others.slice(83)];
})();

const HELD: readonly HeldCheckout[] = [
  { base: "main", uncommitted: ["armada.yml"], behind: 880, ahead: 0 },
  { base: "main", uncommitted: ["armada.yml", "crates/fleet/src/noticing.rs"], clashing: ["crates/fleet/src/noticing.rs"], behind: 12, ahead: 0 },
  { base: "main", behind: 5, ahead: 2 },
  { base: "main", uncommitted: LOOSE, clashing: LOOSE_CLASHING, behind: 7, ahead: 0 },
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
