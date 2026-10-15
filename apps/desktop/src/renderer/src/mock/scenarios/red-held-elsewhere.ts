// Main red, and another machine holds the claim on it. The band names the holder and its age and
// offers Take over, which asks once. A Dispatch is refused twice over the story: once for a store that
// cannot be reached, once for a claim the hub had not read yet.

import type { FixMain, Outcome } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { holding } from "../holding";
import { FIXTURES, redHeldElsewhere, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";

const refused = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

const NOT_MADE = refused("fleet.main_claim_not_made", "the red was not taken. the claim store could not be reached");
const TAKEN = refused("fleet.main_taken_elsewhere", "Job c41e90a7 on work-mac-mini has held this red for 2m. Take it over to replace that claim");

function heldElsewhere(): Scenario {
  const base = holding("red-held-elsewhere", "Main red, claimed on another machine: Take over, a stale claim, a store that cannot be reached, a claim not yet read", FIXTURES);
  const { first, later } = redHeldElsewhere(repository().root);
  return {
    ...base,
    state: { ...base.state, ...first },
    later,
    behaves: (fleet) => {
      // A Dispatch is refused for the store the first time, goes through the second, and is refused as
      // taken the third. A take over replaces a claim the store already answered for, so it never is.
      let presses = 0;
      return {
        ...writingMainsLogs(fleet),
        fixMain: async (fix: FixMain) => {
          if (fix.take_over === true) return { ok: true };
          presses += 1;
          return presses === 1 ? NOT_MADE : presses === 3 ? TAKEN : { ok: true };
        },
      };
    },
  };
}

export const s245RedHeldElsewhere: Scenario = heldElsewhere();
