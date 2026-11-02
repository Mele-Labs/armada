// The rows of the list, as the filters, the scope and the search leave them: one reading for the list that draws them and the keys that walk them.

import { useBaysShowing } from "./instrument-mods";
import { useEffect, useMemo } from "react";

import type { Banding } from "./bands";
import { useFound, type Found } from "./find";
import type { Ship, Subject } from "./ship";
import { listOf, placeNew, useDeck } from "./store";

export function useListed(ship: Ship, banding: readonly Banding[] | undefined): { listed: Subject[]; found: ReadonlyMap<string, Found> | undefined } {
  const checks = useDeck((deck) => deck.checks);
  const held = useDeck((deck) => deck.held);
  const scope = useDeck((deck) => deck.scope);
  const order = useDeck((deck) => deck.order);
  const query = useDeck((deck) => deck.query);
  const open = useDeck((deck) => (deck.sel?.kind === "session" ? deck.sel.key : undefined));
  const found = useFound(query);
  // A Session new to the order takes the bottom of it, and stays there.
  useEffect(() => placeNew(ship), [ship]);
  const bays = useBaysShowing();
  const listed = useMemo(() => listOf(ship, { checks, held, scope, order, bays, ...(found === undefined ? {} : { found }), ...(open === undefined ? {} : { open }) }, banding), [ship, checks, held, scope, order, found, open, banding, bays]);
  return { listed, found };
}
