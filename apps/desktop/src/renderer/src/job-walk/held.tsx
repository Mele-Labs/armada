// The notes held on a Job, as the Job's thread draws them and as the walk's bar sends them. The region is `HeldNotes`, the one the walk window's panel draws; it is the thread's trailing
// entry, so a walk beside the thread opens nothing to show it. `docs/practices/capture-window.md`, *Pending notes*.

import type { ReactNode } from "react";
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

/**
 * Sends every note the Job holds as one turn, words alone and numbered in the order taken, and leaves each of the Job's walks reading still changing until it shows the page again.
 * A Job holding none sends nothing.
 */
export function useSendHeld(session: Session | undefined): () => Promise<HeldAnswer> {
  const draft = useSessionsDraft();
  return async () => {
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
  const notes = useHeldNotes(session?.id);
  const send = useSendHeld(session);
  if (session === undefined || notes.length === 0) return undefined;
  return (
    <HeldNotes
      notes={notes}
      onEdit={async (id, said) => (editNote(session.id, id, said) ? OK : GONE)}
      onRemove={async (id) => (removeNote(session.id, id) ? OK : GONE)}
      onSend={send}
    />
  );
}
