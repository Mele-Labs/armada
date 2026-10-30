// Ending a Job lists first what it still holds in flight (`session-view/InFlightDialog.tsx`), and a Job holding nothing ends at once.
// **Every End asks here**: the Job's menu and its done band, the Cockpit's End job on a done call and the Deck row's Close, so a Job is
// ended the same way from each view of it. The dialog is mounted once with the Sessions; the press is `askToEnd`.

import { useEffect, useSyncExternalStore } from "react";
import { createPortal } from "react-dom";
import { inFlight } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { InFlightDialog } from "./session-view/InFlightDialog";

const NONE = () => () => undefined;
const NO_SESSIONS: readonly Session[] = [];
const EMPTY = () => NO_SESSIONS;

/** The Job whose End is waiting on the person, the Sessions it is a Job of, and what to do once it has ended. */
type Asked = { draft: SessionsDraft; id: string; then: (() => void) | undefined };
let asked: Asked | undefined;
const looks = new Set<() => void>();

const ask = (next: Asked | undefined): void => {
  asked = next;
  for (const look of looks) look();
};

/** What Job `id` of `sessions` still holds in flight. Nothing for a Job whose lanes are not read, and for a mock that carries none. */
const flyingOf = (sessions: readonly Session[], id: string) => {
  const lanes = sessions.find((one) => one.id === id)?.lanes;
  return lanes === undefined ? [] : inFlight(lanes.job);
};

/**
 * End Job `id`: at once when it holds nothing in flight, else after the person has seen the list and pressed End job.
 * `then` runs when it has been ended, either way; a Keep going never runs it.
 */
export function askToEnd(draft: SessionsDraft, id: string, then?: () => void): void {
  if (draft.close === undefined) return;
  if (flyingOf(draft.get(), id).length === 0) {
    draft.close(id);
    then?.();
    return;
  }
  ask({ draft, id, then });
}

/** Drop the ask for Job `id`, as its view leaves the screen: a list for a Job nobody is looking at is one nobody can answer. */
export function dropAsk(id: string): void {
  if (asked?.id === id) ask(undefined);
}

/**
 * The list over whatever is on screen. It shuts when the Job holds nothing in flight any more, ended or not.
 * **Only the mount of the Sessions the press was made on answers it**: the mock nests one served mount in another that serves none, and an
 * ask is one for a draft, not for whichever list is drawn.
 */
export function EndsInFlight({ draft }: { draft: SessionsDraft | undefined }) {
  const sessions = useSyncExternalStore(draft?.subscribe ?? NONE, draft?.get ?? EMPTY);
  const asking = useSyncExternalStore(
    (look) => (looks.add(look), () => void looks.delete(look)),
    () => asked,
  );
  const waiting = draft !== undefined && asking?.draft === draft ? asking : undefined;
  const rows = waiting === undefined ? [] : flyingOf(sessions, waiting.id);
  useEffect(() => {
    if (waiting !== undefined && rows.length === 0) ask(undefined);
  }, [waiting, rows.length]);
  if (waiting === undefined || draft === undefined || rows.length === 0) return null;
  // On the body, appended as it opens: a side panel is a layer at the same height, and the one opened last is drawn over the other.
  return createPortal(
    <InFlightDialog
      rows={rows}
      onEnd={() => {
        ask(undefined);
        draft.close?.(waiting.id);
        waiting.then?.();
      }}
      onKeep={() => ask(undefined)}
    />,
    document.body,
  );
}
