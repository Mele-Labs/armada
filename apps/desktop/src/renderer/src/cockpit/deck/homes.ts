// Where a call lives when no Session's row holds it. A Session's call pulses its row in Work; every other call pulses the one place
// that already shows what it is about, in the same tint and pulse. Nothing is drawn for it and nothing counts it.
//
//   main is red            `main:<root>`      main's light in the merge queue
//   a held checkout        `checkout:<root>`  the same light, which carries the checkout mark
//   a pull request         `pull:<number>`    that pull request's chip in the merge queue (failing, unmergeable, or merge by hand)

import type { Item } from "../../Dashboard";
import { incomingOf } from "./store";

/** The home a call pulses, or none where a Session's row holds it. */
export function homeOf(item: Item): string | undefined {
  return /^(main|checkout|pull):/.test(item.key) && item.ownedBy === undefined
    ? item.key
    : undefined;
}

/** The homes with a call still to open, by the ids above. A call opened or answered has stopped calling, as its row's would. */
export function callingHomes(
  calls: readonly Item[],
  held: ReadonlyMap<string, number>,
  visited: ReadonlySet<string>,
): ReadonlySet<string> {
  return new Set(incomingOf(calls, held, visited).flatMap((one) => homeOf(one) ?? []));
}
