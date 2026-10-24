// A background act that failed after its reply, as main holds it: `act.failed` passed on to every window.
// Nothing is kept here; the refusal is a toast, and a window that was not open does not see it.

import type { ActFailed } from "@armada/protocol";

export class ActFailures {
  private told: (failed: ActFailed) => void = () => {};

  /** Say where a failure goes: main's, which owns the windows. */
  onFailed(told: (failed: ActFailed) => void): void {
    this.told = told;
  }

  /** `act.failed`. */
  failed(failed: ActFailed): void {
    this.told(failed);
  }
}
