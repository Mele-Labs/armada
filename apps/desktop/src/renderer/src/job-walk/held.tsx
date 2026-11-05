// The notes held on a Job, as the Job's thread draws them and as the walk's bar sends them. The region is `HeldNotes`, the one the walk window's panel draws; it is the thread's trailing
// entry, so a walk's tab opens nothing to show it. `docs/practices/capture-window.md`, *Pending notes*.

import { useEffect, useState, type ReactNode } from "react";
import type { Outcome } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";
import type { PanelHeldNote } from "@armada/studios/api";
import { useSessionsDraft } from "../sessions-draft";
import { HeldNotes, type HeldAnswer } from "../walk-panel/HeldNotes";
import { editNote, removeNote, takeHeld, useHeldNotes, workingOn } from "./store";
import { walksOf } from "./walks";
import type { Session } from "@armada/screens/src/draft/sessions";

const GONE: HeldAnswer = {
  ok: false,
  said: "That note is no longer pending.",
};
const OK: HeldAnswer = {
  ok: true,
};

const answered = (outcome: Outcome): HeldAnswer =>
  outcome.ok
    ? OK
    : {
        ok: false,
        said: said(outcome),
      };

/**
 * On a real Bridge main keeps each Job's notes and tells them as they change; the mock keeps them in `store.ts`. The two are one list to the thread.
 */
function useJobHeld(jobId: string | undefined): readonly PanelHeldNote[] {
  const live = window.armada?.jobWalk;
  const mock = useHeldNotes(live === undefined ? jobId : undefined);
  const [told, setTold] = useState<readonly PanelHeldNote[]>([]);
  useEffect(() => {
    setTold([]);
    return live === undefined || jobId === undefined ? undefined : live.held(jobId, setTold);
  }, [
    live,
    jobId,
  ]);
  return live === undefined ? mock : told;
}

/**
 * Sends every note the Job holds as one turn, words alone and numbered in the order taken, and leaves each of the Job's walks reading still changing until it shows the page again.
 * A Job holding none sends nothing.
 */
export function useSendHeld(session: Session | undefined): () => Promise<HeldAnswer> {
  const draft = useSessionsDraft();
  return async () => {
    const live = window.armada?.jobWalk;
    if (session !== undefined && live !== undefined)
      return answered(await live.sendNotes(session.id));
    if (session === undefined || draft === undefined) return OK;
    const notes = takeHeld(session.id);
    if (notes.length === 0) return OK;
    await draft.send(session.id, {
      text: notes.map((one, at) => `${at + 1}. ${one.said}`).join("\n"),
      files: [],
      sketches: [],
      tags: [],
    });
    workingOn(
      session.id,
      walksOf(session).map((one) => one.id),
    );
    return OK;
  };
}

/** The Job's pending notes as the thread's trailing entry; `undefined` while it holds none. */
export function useTrailingNotes(session: Session | undefined): ReactNode {
  const notes = useJobHeld(session?.id);
  const send = useSendHeld(session);
  if (session === undefined || notes.length === 0) return undefined;
  const live = window.armada?.jobWalk;
  return (
    <HeldNotes
      notes={notes}
      onEdit={async (id, words) =>
        live === undefined
          ? editNote(session.id, id, words)
            ? OK
            : GONE
          : answered(await live.editNote(session.id, id, words))
      }
      onRemove={async (id) =>
        live === undefined
          ? removeNote(session.id, id)
            ? OK
            : GONE
          : answered(await live.removeNote(session.id, id))
      }
      onSend={send}
    />
  );
}
