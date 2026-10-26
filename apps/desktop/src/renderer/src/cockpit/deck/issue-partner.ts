// The chip under the pointer or the focus in the Issue line or the Merge line, so its partner in the other line can be lit: an issue's pull requests, or
// the issues a pull request closes. The lines are siblings in the Deck and share nothing else.

import { useSyncExternalStore } from "react";

export type Lit = { kind: "issue"; pulls: readonly number[] } | { kind: "pull"; number: number } | undefined;

let lit: Lit;
const hear = new Set<() => void>();

/** Light the partners of a chip, or none. */
export function light(next: Lit): void {
  lit = next;
  hear.forEach((one) => one());
}

export const useLit = (): Lit =>
  useSyncExternalStore(
    (listener) => (hear.add(listener), () => void hear.delete(listener)),
    () => lit,
  );
