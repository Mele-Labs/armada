// A whole-panel take of the Operator, offered as a variation of one walk: its own Discussion and Queue bodies over the same store.
// Each take is one file in this folder exporting `take`; `?take=<name>` in the address draws it, and no take draws the panel as built.

import type { ComponentType } from "react";

import type { OperatorStore } from "../store";

export type OperatorTake = {
  /** The name in `?take=`. */
  name: string;
  /** Absent keeps the panel's own Discussion. */
  Talk?: ComponentType<{ store: OperatorStore }>;
  Queue: ComponentType<{ store: OperatorStore }>;
};

const FILES = import.meta.glob<{ take?: OperatorTake }>("./*.tsx", {
  eager: true,
});

const asked =
  typeof window === "undefined" ? null : new URLSearchParams(window.location.search).get("take");

/** The take named by the scenario, else by the address, if one of this folder's files is it. */
export function takeNamed(name: string | undefined): OperatorTake | undefined {
  // The board is the Queue he chose; a scenario naming another, or none that exists, draws that or the list.
  const wanted = name ?? asked ?? "board";
  return Object.values(FILES)
    .map((one) => one.take)
    .find((one) => one !== undefined && one.name === wanted);
}
