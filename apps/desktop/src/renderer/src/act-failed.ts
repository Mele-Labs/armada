// A background act that failed after its reply, shown as the toast a refusal shows: Fleet's own words,
// with the act and what it was for carried as the refusal's fields.

import type { ActFailed } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";
import type { RefusedAct } from "./tell";
import { askToTell } from "./tell";

/** What the toast says, and the act it was for. */
export function actFailedToTell({ act, manifest_id, session_id, error }: ActFailed): { sentence: string; act: RefusedAct } {
  return {
    sentence: refusalWords({ ok: false, why: "refused", error }),
    act: {
      act,
      request: {
        code: error.code,
        ...(manifest_id === undefined ? {} : { manifest_id }),
        ...(session_id === undefined ? {} : { session_id }),
      },
    },
  };
}

/** Tells each failure Fleet sends. `window.armada` in the app, a fake in a test. */
export function tellActFailures(
  fleet: { onActFailed: (on: (failed: ActFailed) => void) => () => void },
  tell: (sentence: string, act: RefusedAct) => void = askToTell,
): () => void {
  return fleet.onActFailed((failed) => {
    const { sentence, act } = actFailedToTell(failed);
    tell(sentence, act);
  });
}
