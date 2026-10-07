// A note sent to Fleet, #1250: proposed as a Job through the describe-the-work
// path the composer uses, so it lands at the approval gate like any request and
// nothing runs until a person approves it.

import { said } from "@armada/screens/src/copy";
import type { Outcome, StagedAttachment } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import { requestOf, type Annotation, type Box, type Sent } from "../../../shared/annotations";
import type { Sink } from "./sink";

/** The two calls a send makes on Fleet's seam. */
export type Proposer = Pick<BridgeApi, "stageAttachment" | "proposeFromRequest">;

export type SendAnswer = { ok: true; sent: Sent } | { ok: false; saying: string };

/** What Fleet's refusal says: its coded copy, else the message it sent, else that it refused. */
function refusal(outcome: Outcome): string {
  const coded = said(outcome);
  if (coded !== "") return coded;
  return !outcome.ok && outcome.why === "refused" && outcome.error.message.trim() !== "" ? outcome.error.message.trim() : "Fleet refused it";
}

/** Why Send is not offered here, or null where it is. */
export function unsendable(sink: Sink): string | null {
  return sink.via === "main" ? null : "Sending needs Bridge and its Fleet; in the mock the note stays a file";
}

/**
 * Stage a screenshot of where the note points, where one can be taken, and
 * propose the note's request against the repository the notes are about.
 * **Nothing is saved here**: the caller writes `sent` onto the note, so a send
 * that fails leaves the note exactly as it was.
 */
export async function sendToFleet(note: Annotation, box: Box, sink: Sink, fleet: Proposer, at: Date): Promise<SendAnswer> {
  const root = await sink.root();
  if (root === null) return { ok: false, saying: "Job not sent: no repository found above Bridge" };

  const attachments: StagedAttachment[] = [];
  const png = await sink.capture(box).catch(() => null);
  if (png !== null) {
    const filename = `annotation-${note.id}.png`;
    const { path } = await fleet.stageAttachment(png, filename, "image/png");
    attachments.push({ path, filename, mimeType: "image/png" });
  }

  const answer = await fleet.proposeFromRequest(requestOf(note), attachments, root);
  if (!answer.ok) return { ok: false, saying: `Job not sent: ${refusal(answer.outcome)}` };
  const job = answer.jobs[0];
  if (job === undefined) return { ok: false, saying: "Job not sent: Fleet answered with none" };
  return { ok: true, sent: { jobId: job.id, handle: job.handle, at: at.toISOString() } };
}

/** The calls a Session send makes on Fleet's seam. */
export type SessionSender = Pick<BridgeApi, "startSession" | "sendSessionMessage">;

/** A Session the notes go to, by the title the person knows it by. */
export type SessionTarget = { id: string; title: string };

const base64 = (bytes: ArrayBuffer): string => {
  const view = new Uint8Array(bytes);
  let binary = "";
  for (let at = 0; at < view.length; at += 0x8000) binary += String.fromCharCode(...view.subarray(at, at + 0x8000));
  return btoa(binary);
};

/** A new Session's name: the first note's first line, cut short. */
export const titleOf = (notes: readonly Annotation[]): string => (notes[0]?.text.split("\n")[0] ?? "").trim().slice(0, 60);

/**
 * The notes as one message to a Session, each with its screenshot attached: to `target`, or to a
 * Session started for them where `target` is null. **Nothing is saved here**, as `sendToFleet`.
 * A Session Fleet refuses, a terminal one among them, comes back as its words.
 */
export async function sendToSession(
  notes: readonly { note: Annotation; box: Box }[],
  target: SessionTarget | null,
  sink: Sink,
  fleet: SessionSender,
  at: Date,
): Promise<SendAnswer> {
  const attachments: { name: string; media_type: string; data: string }[] = [];
  for (const { note, box } of notes) {
    const png = await sink.capture(box).catch(() => null);
    if (png !== null) attachments.push({ name: `annotation-${note.id}.png`, media_type: "image/png", data: base64(png) });
  }

  let into = target;
  if (into === null) {
    // On the notes' repository, which the window's pick may not be: on All repositories it is none.
    const root = await sink.root();
    if (root === null) return { ok: false, saying: "Session not started: no repository found above Bridge" };
    const title = titleOf(notes.map((one) => one.note));
    const started = await fleet.startSession(title, root);
    if (!started.ok) return { ok: false, saying: `Session not started: ${refusal(started.outcome)}` };
    into = { id: started.value.id, title: started.value.title ?? title };
  }

  const text = notes.map((one) => requestOf(one.note)).join("\n\n---\n\n");
  const sent = await fleet.sendSessionMessage({ session_id: into.id, text, ...(attachments.length === 0 ? {} : { attachments }) });
  if (!sent.ok) return { ok: false, saying: `Notes not sent to "${into.title}": ${refusal(sent.outcome)}` };
  return { ok: true, sent: { sessionId: into.id, title: into.title, at: at.toISOString() } };
}
