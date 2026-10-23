// Which arrangement a page opens in, remembered across a restart — the Cockpit's
// view, filters and scope (owner, 25 Sep 2026).
//
// **One module, because it is one mechanism.** A second way of remembering a way of
// reading is two places to keep in step.
//
// **`localStorage`, not a Fleet preference**, for `panel-open.ts`' own reason:
// Fleet's preference set is closed (`fleet.unknown_preference`) and a SQL
// `CHECK` constraint names every legal key, so a new one is a wire change in
// five crates. These are ways of reading, not facts about the work.

// ---- The flight deck's views, which live in a store and not in a component ---------------------------------------
//
// The deck keeps its state in a module so the top bar and the body, two trees, agree and a body that unmounts while a
// page is open comes back to the exact view (`cockpit/deck/store.ts`). It remembers the few ways of reading that
// should survive a restart through the same failed-read-is-the-default rule.

export const DECK_VIEW_KEY = "armada.bridge.deck-view";
export const DECK_FILTERS_KEY = "armada.bridge.deck-filters";
/** Which Sessions the list holds besides the live ones: Active, Quiet, Ended or All. */
export const DECK_SCOPE_KEY = "armada.bridge.deck-scope";
/** The order he has placed the live Sessions in, as a JSON list of their keys: kept per machine, since a Session is. */
export const DECK_ORDER_KEY = "armada.bridge.deck-order";
/** The System page the System tab opens on: the last one used. */
export const DECK_SYSTEM_KEY = "armada.bridge.deck-system";

/** What was stored under `key`, on whatever the reader makes of it. A failed read is the default. */
export function readRemembered<T extends string>(key: string, named: (value: string | null) => T): T {
  try {
    return named(window.localStorage.getItem(key));
  } catch {
    return named(null);
  }
}

/** Stores a choice. A failed write leaves it unremembered, the honest answer for a preference. */
export function writeRemembered(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Unremembered.
  }
}
