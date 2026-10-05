// What `rescue_slot` came to, as the screen and the preload both read it.
// Apart from `SlotPools.tsx` so the main process can name the type: a `.tsx`
// import drags `--jsx` into a build that draws nothing.

import type { Outcome, SlotRescued } from "@armada/protocol";

/**
 * The receipt where it was done, a refusal where not. **Apart from `Outcome`**,
 * which a receipt would widen for every command: a Scrap's `branch_kept` is not
 * derivable from `ok`.
 */
export type RescueOutcome = Exclude<Outcome, { ok: true }> | { ok: true; rescued: SlotRescued };
