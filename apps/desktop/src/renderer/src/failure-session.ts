// Dispatch session on a failure toast: a Session started in Armada's own repository whose first message is the
// failure's debug info, so one press stands where copying, opening a Session and pasting did. It starts the way a
// Session's Report does (`draft.start` in `reportRoot`, then `draft.send`), and opens it the way every hand-off does.
// A start Fleet refused is said as a toast, since the toast that offered it may be over a page that shows no refusal.
// The owner's ask of 15 Oct 2026.

import { useRef } from "react";
import type { RepositorySummary } from "@armada/protocol";

import { plain, reportRoot } from "./cockpit/deck/session-tiles";
import { useSessionsDraft } from "./sessions-draft";
import { askToTell } from "./tell";

/**
 * What a failure toast calls to hand its debug info to a new Session, or `undefined` where it cannot: nothing serves
 * Sessions, or Fleet serves no Armada repository to start one in. It resolves to whether the Session was started and told.
 */
export function useDispatchFailure(repositories: readonly RepositorySummary[], open: (id: string) => void): ((info: string) => Promise<boolean>) | undefined {
  const draft = useSessionsDraft();
  /** The Session a press started, kept while its message is refused so a second press tells the same one. */
  const started = useRef<string | undefined>(undefined);
  const armada = reportRoot(repositories, undefined);
  if (draft === undefined || armada === undefined) return undefined;
  return async (info: string): Promise<boolean> => {
    const id = started.current ?? (await draft.start(undefined, armada));
    if (id === undefined) {
      const why = draft.said?.();
      askToTell(why === undefined ? "Session not started" : `Session not started: ${why}`);
      return false;
    }
    started.current = id;
    if ((await draft.send(id, plain(info))) === false) return false;
    started.current = undefined;
    open(id);
    return true;
  };
}
