// Debug Bridge's Session: started with nothing sent. Its note and picture wait in the message box as a
// draft, so the owner adds his context and Enter is the first turn. Fleet starts a Session without one.

import { writeKept, type ComposerFile } from "@armada/components";

import { requestOf, type Annotation, type Box } from "../../../shared/annotations";
import { pictureOf, refusal, type SendAnswer, type SessionSender } from "./send";
import type { Sink } from "./sink";

/** The message box's own key for a Session (`SessionComposer`'s `draftKey`). */
const keptFor = (sessionId: string): string => `session:${sessionId}`;

/** `text` and `picture` in the box kept for the Session, as though typed and attached there. */
function leaveDraft(sessionId: string, text: string, picture: ArrayBuffer | null, name: string): void {
  // A line break that ends the box is only where the last line stops, so one more keeps a blank line under the state.
  const seeded = `${text}\n\n`;
  const file = picture === null ? undefined : new File([picture], name, { type: "image/png" });
  const files: ComposerFile[] = file === undefined ? [] : [{ id: "debug-shot", name, file, src: URL.createObjectURL(file) }];
  writeKept(keptFor(sessionId), { text: seeded, tags: [], files }, { text: seeded, tags: [] });
}

/**
 * A Session started on the note's repository, holding the note and its picture as a draft and no turn.
 * **Nothing is saved or sent here**: the caller keeps the note, and the Session hears nothing until a person sends.
 */
export async function startWithDraft(
  note: Annotation,
  box: Box | null,
  sink: Sink,
  fleet: Pick<SessionSender, "startSession">,
): Promise<Extract<SendAnswer, { ok: false }> | { ok: true; sessionId: string }> {
  const root = await sink.root();
  if (root === null) return { ok: false, saying: "Job not started: no repository found above Bridge" };
  const picture = await pictureOf(note, box, sink);
  const words = requestOf(note);
  // Held on its start card, with the note's words as the request so Fleet's proposer reads them at the start.
  const started = await fleet.startSession(undefined, root, words, undefined, undefined, true);
  if (!started.ok) return { ok: false, saying: `Job not started: ${refusal(started.outcome)}` };
  leaveDraft(started.value.id, words, picture?.png ?? null, `annotation-${note.id}.png`);
  return { ok: true, sessionId: started.value.id };
}
