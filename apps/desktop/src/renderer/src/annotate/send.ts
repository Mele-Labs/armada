// A note sent to Fleet, #1250: to a Session, a new one or one that is live, as a pinned note.

import { DEBUG_INFO_HEADING } from "@armada/components";
import { said } from "@armada/screens/src/copy";
import type { Outcome } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import { requestOf, type Annotation, type Box, type Sent } from "../../../shared/annotations";
import { annotationNoteBody } from "../../../shared/pinned-note";
import type { Sink } from "./sink";

/** Refused partway, `sentIds` are the notes Fleet took before it refused, all to `sent`. */
export type SendAnswer = { ok: true; sent: Sent } | { ok: false; saying: string; sent?: Sent; sentIds?: string[] };

/** What Fleet's refusal says: its coded copy, else the message it sent, else that it refused. */
export function refusal(outcome: Outcome): string {
  const coded = said(outcome);
  if (coded !== "") return coded;
  return !outcome.ok && outcome.why === "refused" && outcome.error.message.trim() !== "" ? outcome.error.message.trim() : "Fleet refused it";
}

/**
 * The note's picture: the one kept as it was saved, else one taken now where its element is on
 * this screen, else none. Only a note saved before pictures were kept, or whose file is gone, falls back.
 */
export async function pictureOf(note: Annotation, box: Box | null, sink: Sink): Promise<ArrayBuffer | null> {
  if (note.screenshot !== undefined) {
    const kept = await sink.shot(note.id).catch(() => null);
    if (kept !== null) return kept;
  }
  // A null box is a note whose element is not on this screen: a shot of the screen would be of the wrong thing.
  return box === null ? null : sink.capture(box).catch(() => null);
}

/** Why Send is not offered here, or null where it is. */
export function unsendable(sink: Sink): string | null {
  return sink.via === "main" ? null : "Sending needs Bridge and its Fleet; in the mock the note stays a file";
}

/** The calls a Session send makes on Fleet's seam. */
export type SessionSender = Pick<BridgeApi, "startSession" | "sendSessionMessage" | "sendSessionNote">;

/** A Session the notes go to, by the title the person knows it by. */
export type SessionTarget = { id: string; title: string };

const base64 = (bytes: ArrayBuffer): string => {
  const view = new Uint8Array(bytes);
  let binary = "";
  for (let at = 0; at < view.length; at += 0x8000) binary += String.fromCharCode(...view.subarray(at, at + 0x8000));
  return btoa(binary);
};

/** The Job open in the window when the notes were sent, and its labelled fact lines as its pane gave them (`job-facts.ts`). */
export type OpenJob = { id: string; facts: readonly string[] };

/** What the facts message says before them: where they are from, so the agent reads them as the Job the notes were pinned on. */
const FACTS_LEAD = "The owner pinned the notes that follow in Bridge with this Job open. What follows is what Bridge held about that Job, as Report gives it.";

/** A new Session's name: the first note's first line, cut short. */
export const titleOf = (notes: readonly Annotation[]): string => (notes[0]?.text.split("\n")[0] ?? "").trim().slice(0, 60);

/**
 * Each note as its own `send_session_note`, its picture as the crop (`pictureOf`): to `target`, or to a Session
 * started for them where `target` is null. `address` is the page's url. **A whole-window note stays a message**
 * (`requestOf`), since what Bridge held has no place in a pinned note. **Nothing is saved here**, as `sendToFleet`.
 * A Session Fleet refuses, a terminal one among them, comes back as its words.
 *
 * **With a Job open** (`open`), its facts go first as one message, unless they are for the Job the notes are going to, which holds them.
 */
export async function sendToSession(
  notes: readonly { note: Annotation; box: Box | null }[],
  target: SessionTarget | null,
  sink: Sink,
  fleet: SessionSender,
  address: string,
  at: Date,
  open?: OpenJob,
): Promise<SendAnswer> {
  const crops: (string | null)[] = [];
  for (const { note, box } of notes) {
    const png = await pictureOf(note, box, sink);
    crops.push(png === null ? null : base64(png));
  }

  let into = target;
  if (into === null) {
    // On the notes' repository, which the window's pick may not be: on All repositories it is none.
    const root = await sink.root();
    if (root === null) return { ok: false, saying: "Job not started: no repository found above Bridge" };
    const title = titleOf(notes.map((one) => one.note));
    const started = await fleet.startSession(title, root);
    if (!started.ok) return { ok: false, saying: `Job not started: ${refusal(started.outcome)}` };
    into = { id: started.value.id, title: started.value.title ?? title };
  }

  const sent: Sent = { sessionId: into.id, title: into.title, at: at.toISOString() };
  const sentIds: string[] = [];
  if (open !== undefined && open.facts.length > 0 && open.id !== into.id) {
    const facts = await fleet.sendSessionMessage({ session_id: into.id, text: [FACTS_LEAD, "", DEBUG_INFO_HEADING, ...open.facts].join("\n") });
    if (!facts.ok) return { ok: false, saying: `Notes not sent to "${into.title}": ${refusal(facts.outcome)}` };
  }
  for (const [index, { note }] of notes.entries()) {
    const crop = crops[index] ?? null;
    const answer =
      note.scope === "window"
        ? await fleet.sendSessionMessage({
            session_id: into.id,
            text: requestOf(note),
            ...(crop === null ? {} : { attachments: [{ name: `annotation-${note.id}.png`, media_type: "image/png", data: crop }] }),
          })
        : await fleet.sendSessionNote(annotationNoteBody(note, into.id, address, crop));
    if (!answer.ok) {
      const saying = `Notes not sent to "${into.title}": ${refusal(answer.outcome)}`;
      return sentIds.length === 0 ? { ok: false, saying } : { ok: false, saying, sent, sentIds };
    }
    sentIds.push(note.id);
  }
  return { ok: true, sent };
}
