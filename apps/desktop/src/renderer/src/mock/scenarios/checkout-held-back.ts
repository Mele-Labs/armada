// The served checkout is behind main and Fleet would not catch it up: the call it raises, over four holds. Each is its
// own moment on one repository's hub, so the card changes in place; the one that is safe to press runs and goes. Drawn
// ahead of Fleet in `checkout-held-hub.ts`; the walk `checkoutHeldBack` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";

import { checkoutHeld } from "../checkout-held-hub";
import { holding } from "../holding";
import { FIXTURES } from "../main-red-hub";
import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

function held(): Scenario {
  const base = holding("checkout-held-back", "The served checkout behind main, held by uncommitted files, another branch or a diverged history", FIXTURES);
  const { first, later } = checkoutHeld(repository().root);
  return {
    ...base,
    // A quiet Board: the call is the only thing asking.
    state: { ...base.state, ...first, jobs: [] },
    later,
    draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, []) },
  };
}

export const s256CheckoutHeldBack: Scenario = held();
