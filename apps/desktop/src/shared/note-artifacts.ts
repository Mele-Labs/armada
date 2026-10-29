// A note sent to a Job, as Fleet keeps it on that Job: an attachment of kind `artifact`, form
// `note`, target `note:<id>`. Every detail value is a string, as the ledger keeps them; this reads
// them into a type. Fleet's half is `crates/fleet/src/session_host/annotating.rs`.

import type { Attachment, SessionRecord } from "@armada/protocol";

export const NOTE_FORM = "note";

/** The file's `open` or `done`, or `returned` where the Job closed unfinished and the note went back to no Job. */
export type NoteArtifactStatus = "open" | "done" | "returned";

export type NoteArtifact = {
  /** The note's id, which names its file. */
  note: string;
  /** The first line of his words, cut to 80 characters. */
  title: string;
  text: string;
  /** The element's tag. */
  tag: string;
  component?: string;
  /** The rail item marked current when he wrote it. */
  screen?: string;
  /** The page's address, on a pinned note. */
  address?: string;
  /** The note's file, absolute, where Fleet found it. */
  file?: string;
  /** Its picture, absolute, where it has one. Never inline. */
  picture?: string;
  status: NoteArtifactStatus;
  /** When Fleet kept it, RFC3339. */
  sent: string;
};

const STATUSES: readonly string[] = ["open", "done", "returned"];

/** The note an attachment keeps, or null where it is not a note artifact. */
export function noteArtifactOf(attachment: Attachment): NoteArtifact | null {
  const detail = attachment.detail ?? {};
  if (attachment.kind !== "artifact" || detail["form"] !== NOTE_FORM) return null;
  const { note, title, text, tag, status, sent } = detail;
  if (note === undefined || text === undefined || tag === undefined || sent === undefined) return null;
  if (status === undefined || !STATUSES.includes(status)) return null;
  const optional = (key: "component" | "screen" | "address" | "file" | "picture") =>
    detail[key] === undefined ? {} : { [key]: detail[key] };
  return {
    note,
    title: title ?? text,
    text,
    tag,
    ...optional("component"),
    ...optional("screen"),
    ...optional("address"),
    ...optional("file"),
    ...optional("picture"),
    status: status as NoteArtifactStatus,
    sent,
  };
}

/** Every note a Job keeps, in the order its record lists them. */
export const noteArtifactsOf = (record: Pick<SessionRecord, "attachments">): NoteArtifact[] =>
  record.attachments.map(noteArtifactOf).filter((one): one is NoteArtifact => one !== null);

/**
 * Whether a Session list shows Fleet rewrote a note file since the last list: a note artifact's
 * status moved, or a Job ended, whose close marks its notes done or lets them go. Main holds one
 * and tells the annotation layer, which reads its files again.
 */
export class NotesWatch {
  private readonly seen = new Map<string, string>();

  read(sessions: readonly Pick<SessionRecord, "id" | "state" | "attachments">[]): boolean {
    let touched = false;
    for (const session of sessions) {
      const notes = noteArtifactsOf(session);
      const key = `${session.state}|${notes.map((one) => `${one.note}=${one.status}`).join(",")}`;
      const was = this.seen.get(session.id);
      if (was !== key) {
        // A Job first met counts only where it holds notes; one met live that ends always does.
        touched ||= was === undefined ? notes.length > 0 : true;
        this.seen.set(session.id, key);
      }
    }
    return touched;
  }
}
