// A note sent to Fleet, #1250: to a Session, a new one or one that is live, as a pinned note.

import { said } from "@armada/screens/src/copy";
import type { Outcome } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import { requestOf, type Annotation, type Box, type Sent } from "../../../shared/annotations";
import { annotationNote } from "../../../shared/pinned-note";
import { pngSize, ringOn, type Ring } from "../../../shared/ring";
import { windowBox } from "./capture";
import type { Sink } from "./sink";

/** Refused partway, `sentIds` are the notes Fleet took before it refused, all to `sent`. */
export type SendAnswer = { ok: true; sent: Sent } | { ok: false; saying: string; sent?: Sent; sentIds?: string[] };

/** What Fleet's refusal says: its coded copy, else the message it sent, else that it refused. */
export function refusal(outcome: Outcome): string {
  const coded = said(outcome);
  if (coded !== "") return coded;
  return !outcome.ok && outcome.why === "refused" && outcome.error.message.trim() !== "" ? outcome.error.message.trim() : "Fleet refused it";
}

/** A note's picture, the whole window, and where its element is on it. No ring on a whole-window note's. */
export type Picture = { png: ArrayBuffer; ring: Ring | null };

/**
 * The note's picture: the one kept as it was saved, else the window taken now where its element is on
 * this screen, else none. Only a note saved before pictures were kept, or whose file is gone, falls back.
 */
export async function pictureOf(note: Annotation, box: Box | null, sink: Sink): Promise<Picture | null> {
  if (note.screenshot !== undefined) {
    const kept = await sink.shot(note.id).catch(() => null);
    if (kept !== null) return { png: kept, ring: note.scope === "window" ? null : (note.ring ?? null) };
  }
  // A null box is a note whose element is not on this screen: a shot of the screen would be of the wrong thing.
  if (box === null) return null;
  const window = windowBox();
  const png = await sink.capture(window).catch(() => null);
  if (png === null) return null;
  const size = note.scope === "window" ? null : pngSize(new Uint8Array(png));
  return { png, ring: size === null ? null : ringOn(box, window.width, size.width) };
}

/** Why Send is not offered here, or null where it is. */
export function unsendable(sink: Sink): string | null {
  return sink.via === "main" ? null : "Sending needs Bridge and its Fleet; in the mock the note stays a file";
}

/** The calls a Session send makes on Fleet's seam. */
export type SessionSender = Pick<BridgeApi, "startSession" | "sendSessionMessage" | "sendSessionNotes">;

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

/** What a whole-window note's message says before the Job's facts: they are background, so the agent starts from the owner's words. */
const BACKGROUND_LEAD = "Background, not the task: the Job he had open when he pinned this";

/** A new Session's name: the first note's first line, cut short. */
export const titleOf = (notes: readonly Annotation[]): string => (notes[0]?.text.split("\n")[0] ?? "").trim().slice(0, 60);

/**
 * The notes to `target`, or to a Session started for them where `target` is null. `address` is the page's url. **The notes
 * on an element go as one `send_session_notes` turn**, numbered in their order here, each picture (`pictureOf`) ringed in
 * main with its number, as a walk's are. **A whole-window note stays a message of its own** (`requestOf`), since what Bridge
 * held has no place in a pinned note; it goes where it stands in the list, and the turn where its first note does.
 * **Nothing is saved here**, as `sendToFleet`. A Session Fleet refuses, a terminal one among them, comes back as its words.
 *
 * **With a Job open** (`open`), its facts ride on the first note sent, after his words, unless they are for the Job the notes are going to, which holds them.
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
  const pictures: (Picture | null)[] = [];
  for (const { note, box } of notes) pictures.push(await pictureOf(note, box, sink));

  let into = target;
  if (into === null) {
    // On the notes' repository, which the window's pick may not be: on All repositories it is none.
    const root = await sink.root();
    if (root === null) return { ok: false, saying: "Job not started: no repository found above Bridge" };
    // Held on its start card: nothing runs until he presses Start. His words ride as the request, so Fleet's proposer reads them now.
    const request = notes.map((one) => one.note.text.trim()).filter((words) => words !== "").join("\n");
    // No title goes up: Fleet names the Job from his words, and the notice falls back to the first line until the record says.
    const started = await fleet.startSession(undefined, root, request === "" ? undefined : request, undefined, undefined, true);
    if (!started.ok) return { ok: false, saying: `Job not started: ${refusal(started.outcome)}` };
    into = { id: started.value.id, title: started.value.title ?? titleOf(notes.map((one) => one.note)) };
  }

  const sent: Sent = { sessionId: into.id, title: into.title, at: at.toISOString() };
  const sentIds: string[] = [];
  // Background for the first thing sent alone: the Job is one fact about all of them.
  const job = open !== undefined && open.id !== into.id ? open.facts : [];
  const onElements = notes.flatMap(({ note }, index) => (note.scope === "window" ? [] : [{ note, picture: pictures[index] ?? null }]));
  const firstOnElement = notes.findIndex(({ note }) => note.scope !== "window");
  // What goes, in the notes' order: each whole-window note, and the turn where its first note stands.
  type Send = "turn" | { note: Annotation; picture: Picture | null };
  const sends = notes.flatMap(({ note }, index): Send[] => (note.scope === "window" ? [{ note, picture: pictures[index] ?? null }] : index === firstOnElement ? ["turn"] : []));
  for (const [index, one] of sends.entries()) {
    const facts = index === 0 ? job : [];
    const answer =
      one === "turn"
        ? await fleet.sendSessionNotes({
            session_id: into.id,
            notes: onElements.map(({ note, picture }, at) => ({
              note: annotationNote(note, address, picture === null ? null : base64(picture.png), at === 0 ? facts : []),
              ...(picture === null || picture.ring === null ? {} : { ring: picture.ring }),
            })),
          })
        : await fleet.sendSessionMessage({
            session_id: into.id,
            text: facts.length > 0 ? [requestOf(one.note), "", BACKGROUND_LEAD, ...facts].join("\n") : requestOf(one.note),
            ...(one.picture === null ? {} : { attachments: [{ name: `annotation-${one.note.id}.png`, media_type: "image/png", data: base64(one.picture.png) }] }),
          });
    if (!answer.ok) {
      const saying = `Notes not sent to "${into.title}": ${refusal(answer.outcome)}`;
      return sentIds.length === 0 ? { ok: false, saying } : { ok: false, saying, sent, sentIds };
    }
    sentIds.push(...(one === "turn" ? onElements.map(({ note }) => note.id) : [one.note.id]));
  }
  return { ok: true, sent };
}
