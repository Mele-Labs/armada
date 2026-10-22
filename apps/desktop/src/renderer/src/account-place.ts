// Where a Session's running account and live view draw: in the thread under the message it answers, in a panel beside the thread, or as
// a band on the composer. Three takes the owner is choosing between; a mock scenario names its own, and the thread is the default.

export type AccountPlace = "thread" | "side" | "band";

let place: AccountPlace = "thread";

/** A scenario's take, set as its window is built. */
export function placeAccount(next: AccountPlace): void {
  place = next;
}

export const accountPlace = (): AccountPlace => place;
