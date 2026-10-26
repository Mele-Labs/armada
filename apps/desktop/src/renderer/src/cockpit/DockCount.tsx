// The count on the dock: the calls still to be opened, read from the list the alerts and the title row read
// (`useItems`, then `incomingOf`), so the number is always one he can find. One a Job, however many of its
// items call, as its row pulses once. Main sets the badge; only the window knows which Sessions he has been in.

import { useEffect } from "react";

import type { BridgeState } from "../../../shared/bridge";
import { useItems, type Hosts } from "../Dashboard";
import { incomingOf, useDeck } from "./deck/store";

/** `useItems` draws acts it never calls here, and a stable object keeps its memo. */
const NO_HOSTS: Hosts = { onOpenSession: () => {}, onOpenLink: () => {} };
const NONE: ReadonlySet<string> = new Set();

export function DockCount({ state }: { state: BridgeState }) {
  const held = useDeck((deck) => deck.held);
  const visited = useDeck((deck) => deck.visited);
  const count = incomingOf(useItems("command-central", state, null, NO_HOSTS, NONE), held, visited).length;
  // Not before Fleet has answered once: an empty reading would clear a count that stands.
  useEffect(() => {
    const bridge = window.armada;
    if (state.readAt === null || typeof bridge?.dockCount !== "function") return;
    bridge.dockCount(count);
  }, [count, state.readAt]);
  return null;
}
