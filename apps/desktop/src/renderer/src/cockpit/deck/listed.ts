// The rows of the list, as the filters, the scope and the search leave them: one reading for the list that draws them and the keys that walk them.

import { useMemo } from "react";

import type { Banding } from "./bands";
import { useFound, type Found } from "./find";
import type { Ship, Subject } from "./ship";
import { listOf, useDeck } from "./store";

export function useListed(ship: Ship, banding: readonly Banding[] | undefined): { listed: Subject[]; found: ReadonlyMap<string, Found> | undefined } {
  const needs = useDeck((deck) => deck.needs);
  const checks = useDeck((deck) => deck.checks);
  const held = useDeck((deck) => deck.held);
  const scope = useDeck((deck) => deck.scope);
  const query = useDeck((deck) => deck.query);
  const open = useDeck((deck) => (deck.sel?.kind === "session" ? deck.sel.key : undefined));
  const found = useFound(query);
  const listed = useMemo(() => listOf(ship, { needs, checks, held, scope, ...(found === undefined ? {} : { found }), ...(open === undefined ? {} : { open }) }, banding), [ship, needs, checks, held, scope, found, open, banding]);
  return { listed, found };
}
