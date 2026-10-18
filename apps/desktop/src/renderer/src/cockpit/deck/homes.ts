// Where a call lives when no Session's row holds it. A Session's call pulses its row in Work; every other call pulses the one place
// that already shows what it is about, in the same tint and pulse. Nothing is drawn for it and nothing counts it.
//
//   main is red            `main:<root>`      main's light in the merge queue
//   a held checkout        `checkout:<root>`  the same light, which carries the checkout mark
//   a pull request         `pull:<number>`    that pull request's chip in the merge queue (failing, unmergeable, or merge by hand)
//   Sleep decided          `sleep`            the title row's menu button, where Sleep's entry is
//   Helm's own Session     `helm`             the title row's Helm button, while the dock is shut

import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../../shared/bridge";
import { useItems, type Item } from "../../Dashboard";
import { useSessions } from "../../sessions-draft";
import { sessionIdOf } from "../waiting";
import { incomingOf, useDeck } from "./store";

export const SLEEP_HOME = "sleep";
export const HELM_HOME = "helm";

const NO_HOSTS = { onOpenSession: () => {}, onOpenLink: () => {} };
const NONE: ReadonlySet<string> = new Set();

/** The home a call pulses, or none where a Session's row holds it. */
export function homeOf(item: Item, sessions: readonly Session[]): string | undefined {
  const helm = (id: string | undefined) => (id !== undefined && sessions.find((one) => one.id === id)?.helm === true ? HELM_HOME : undefined);
  if (item.key.startsWith("sleep:")) return SLEEP_HOME;
  if (/^(main|checkout|pull):/.test(item.key)) return item.ownedBy === undefined ? item.key : helm(item.ownedBy.id);
  return helm(sessionIdOf(item.key));
}

/** The homes with a call still to open, by the ids above. A call opened or answered has stopped calling, as its row's would. */
export function callingHomes(calls: readonly Item[], held: ReadonlyMap<string, number>, visited: ReadonlySet<string>, sessions: readonly Session[]): ReadonlySet<string> {
  return new Set(incomingOf(calls, held, visited).flatMap((one) => homeOf(one, sessions) ?? []));
}

/** The same, read where the ship is not: the title row's controls, on every surface. */
export function useCallingHomes(state: BridgeState, picked: RepositorySummary | null): ReadonlySet<string> {
  const held = useDeck((deck) => deck.held);
  const visited = useDeck((deck) => deck.visited);
  const sessions = useSessions();
  const calls = useItems("command-central", state, picked, NO_HOSTS, NONE);
  return callingHomes(calls, held, visited, sessions);
}
