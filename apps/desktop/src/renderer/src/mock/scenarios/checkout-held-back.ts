// The served checkout is behind main and Fleet would not catch it up: the light it raises beside main on the merge queue, over
// the flight deck's ship, in four holds. Each is its own moment on one repository's hub, so the light's card changes in place;
// the safe one is run from the card and the checkout is level. Drawn ahead of Fleet in `checkout-held-hub.ts`; the walk
// `checkoutHeldBack` plays it.

import type { FleetHandle, Scenario } from "../moment";
import { checkoutDiffOfWalk, checkoutHeld, SENT_PULL_REQUEST } from "../checkout-held-hub";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

/** How long Fleet takes to commit the files, push and open the pull request. */
const SEND_AFTER = 900;

/** How long a command that passed stays on the card before Fleet has caught the checkout up. */
const LEVEL_AFTER = 1400;

export const s257CheckoutHeldBack: Scenario = (() => {
  const lines = ship.state.mergeLines!;
  const { first, later, level, sent, safe } = checkoutHeld(lines, lines.lines[0]!.root);
  let fleet: FleetHandle | undefined;
  const levelled = () => {
    if (fleet?.state().mergeLines?.lines[0]?.hub?.checkout?.behind === safe.behind) fleet.publish(level);
  };
  return {
    ...ship,
    name: "checkout-held-back",
    says: "The flight deck with the served checkout behind main, held by uncommitted files, a long list of them or a diverged history",
    state: { ...ship.state, ...first },
    later,
    draft: {
      ...ship.draft,
      sessions: (control) => {
        const store = ship.draft!.sessions!(control);
        return {
          ...store,
          // The ship's own Session reply is not this story's.
          later: () => undefined,
          // Fleet reads a listed file's change from the checkout; the walk has these fixtures.
          diffInRepository: async (_at, path) => ({ ok: true, diff: checkoutDiffOfWalk(path) }),
          // Fleet commits the files to a branch, opens the pull request with auto-merge on and carries it on the hold.
          sendCheckoutAsPullRequest: (_at) =>
            new Promise((done) =>
              window.setTimeout(() => {
                fleet?.publish(sent);
                done({ number: SENT_PULL_REQUEST });
              }, SEND_AFTER),
            ),
          // A command that passes is what brings the checkout level: Fleet reads it a beat later, if the walk has not moved on to another hold.
          runInRepository: (at, command, on) =>
            store.runInRepository!(at, command, (event) => {
              on(event);
              if (event.kind === "end" && event.code === 0) window.setTimeout(levelled, LEVEL_AFTER);
            }),
        };
      },
    },
    behaves: (handle) => {
      fleet = handle;
      return ship.behaves?.(handle) ?? {};
    },
  };
})();
