// The one place that hears every failure card drawn, toast or band, so an app can pass it on without a press.
//
// **Registered, not passed down.** The root boundary draws its card above the tree that would have held a provider, and a
// card drawn there is the one most worth passing on. A listener here hears it too, as `watchUncaught` hears a throw.

import type { FailureFacts } from "./failures";

let listener: ((facts: FailureFacts) => void) | undefined;

/** Hear each failure card as it is drawn. Returns the unsubscribe. */
export function watchFailureShown(onShown: (facts: FailureFacts) => void): () => void {
  listener = onShown;
  return () => {
    if (listener === onShown) listener = undefined;
  };
}

/** A failure card was drawn. A listener that throws never takes the card down with it. */
export function failureShown(facts: FailureFacts): void {
  try {
    listener?.(facts);
  } catch {
    // Passing a card on is never a reason to draw another.
  }
}
