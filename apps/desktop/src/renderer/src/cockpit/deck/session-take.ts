// Three takes on where a selected Session keeps Flight control and the Field, so the thread gets the height. Mock only: the mock page
// sets it from `?take=` (`mock/main.tsx`), and the app proper never does, so it draws the layout as it has always been. `right` runs
// the thread the full height of the middle and stacks Flight control, as a row of icon keys, and the Field in the right column;
// `fold` keeps the bottom row and folds its two panels to their bands until opened; `keys` puts the row of keys under the composer
// and the Field in the right column.

export type SessionTake = "right" | "fold" | "keys";

const TAKES: readonly SessionTake[] = ["right", "fold", "keys"];

let taken: SessionTake | undefined;

/** The take the page asked for; anything else is no take. */
export function setSessionTake(asked: string | null | undefined): void {
  taken = TAKES.find((one) => one === asked);
}

export const sessionTake = (): SessionTake | undefined => taken;
