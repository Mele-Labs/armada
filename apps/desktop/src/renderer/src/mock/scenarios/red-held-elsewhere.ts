// Main red, and another machine holds the claim on it. The band names the holder and its age and
// offers Take over, which asks once. Dispatch is refused once for a store that cannot be reached.
// Two takes: the claim in the Fixing row (`red-held-elsewhere`), or in the band's head (`-b`).

import type { FixMain, Outcome } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { holding } from "../holding";
import { FIXTURES, redHeldElsewhere, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";

const NOT_MADE: Outcome = {
  ok: false,
  why: "refused",
  error: { code: "fleet.main_claim_not_made", message: "the claim store could not be reached", run_id: "mock", fields: {}, chain: [] },
};

function heldElsewhere(name: string, says: string, take: "row" | "head"): Scenario {
  const base = holding(name, says, FIXTURES);
  const { first, later } = redHeldElsewhere(repository().root);
  return {
    ...base,
    state: { ...base.state, ...first },
    later,
    draft: { redClaimTake: take },
    behaves: (fleet) => {
      // The store is out when a Dispatch is first pressed, and back on the next press. A take over is
      // never refused here: it replaces a claim the store already answered for.
      let out = true;
      return {
        ...writingMainsLogs(fleet),
        fixMain: async (fix: FixMain) => {
          if (fix.take_over === true || !out) return { ok: true };
          out = false;
          return NOT_MADE;
        },
      };
    },
  };
}

export const s245RedHeldElsewhere: Scenario = heldElsewhere(
  "red-held-elsewhere",
  "Main red, claimed on another machine: the holder in the Fixing row, Take over, a stale claim, a store that cannot be reached",
  "row",
);

export const s245RedHeldElsewhereB: Scenario = heldElsewhere(
  "red-held-elsewhere-b",
  "Main red, claimed on another machine: the holder in the band's head, where the acts were",
  "head",
);
