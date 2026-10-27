// Notes the owner leaves for the Job that holds a pull request, on lines of its diff. **Fleet keeps them**, on this Mac and never at
// the forge, so the Job a note was sent to can resolve it with a tool. What the window holds here is a copy of what Fleet last
// answered, redrawn as it changes. A note leaves only when he sends the unsent ones to a Job, as one message.

import { useEffect, useSyncExternalStore } from "react";
import type { DiffSide, PullRequestNote } from "@armada/protocol";

import type { PullRequestNotesReads } from "./reads";

export type JobNote = {
  id: string;
  path: string;
  line: number;
  side: DiffSide;
  body: string;
  /** Sent to a Job, which `title` names. */
  sent: boolean;
  /** That Job dealt with it. */
  done: boolean;
  title?: string;
};

/** Which pull request's notes, and how Fleet is asked for them. */
export type NoteHandle = { reads: PullRequestNotesReads; manifestId: string; number: number };

const keyOf = (handle: NoteHandle): string => `${handle.manifestId}#${handle.number}`;
const NONE: readonly JobNote[] = [];

const ofWire = (note: PullRequestNote): JobNote => ({
  id: note.id,
  path: note.path,
  line: note.line,
  side: note.side,
  body: note.body,
  sent: note.sent_to !== undefined,
  done: note.status === "done",
  ...(note.sent_title === undefined ? {} : { title: note.sent_title }),
});

const kept = new Map<string, readonly JobNote[]>();
const listeners = new Set<() => void>();

const keep = (handle: NoteHandle, notes: readonly PullRequestNote[]): void => {
  kept.set(keyOf(handle), notes.map(ofWire));
  listeners.forEach((one) => one());
};

/** What the window holds for a pull request. The same array until the notes change. */
export const readNotes = (handle: NoteHandle): readonly JobNote[] => kept.get(keyOf(handle)) ?? NONE;

/** Drops what the window holds, as a reload does. */
export function forgetNotes(): void {
  kept.clear();
  listeners.forEach((one) => one());
}

/** Reads the notes from Fleet. A refusal leaves what the window held. */
export async function loadNotes(handle: NoteHandle): Promise<void> {
  const answer = await handle.reads.list(handle.manifestId, handle.number).catch(() => undefined);
  if (answer?.ok === true) keep(handle, answer.value.notes);
}

/** Fleet refuses a blank note, and nothing is kept. */
export async function addNote(handle: NoteHandle, note: { path: string; line: number; side: DiffSide; body: string }): Promise<void> {
  const answer = await handle.reads.save(handle.manifestId, handle.number, note).catch(() => undefined);
  if (answer?.ok === true) await loadNotes(handle);
}

/** Fleet refuses an edit of a note that is sent or done. */
export async function editNote(handle: NoteHandle, id: string, body: string): Promise<void> {
  const answer = await handle.reads.save(handle.manifestId, handle.number, { id, body }).catch(() => undefined);
  if (answer?.ok === true) await loadNotes(handle);
}

export async function removeNote(handle: NoteHandle, id: string): Promise<void> {
  const answer = await handle.reads.remove(handle.manifestId, handle.number, id).catch(() => undefined);
  if (answer?.ok === true) keep(handle, answer.value.notes);
}

/** Records the notes as sent to the Job, once the message has gone. Answers whether Fleet took it. */
export async function sendNotes(handle: NoteHandle, ids: readonly string[], to: { id: string; title: string }): Promise<boolean> {
  const answer = await handle.reads.send(handle.manifestId, handle.number, { note_ids: [...ids], session_id: to.id, title: to.title }).catch(() => undefined);
  if (answer?.ok !== true) return false;
  keep(handle, answer.value.notes);
  return true;
}

/** What the Job reads: each note numbered, with the file and line the forge would name it by and the id to resolve it with. */
export function messageOf(number: number, notes: readonly JobNote[]): string {
  const lines = notes.map((one, at) => `${at + 1}. ${one.path}:${one.line}${one.side === "left" ? " on the removed line" : ""} [${one.id}]: ${one.body}`);
  return [`Notes on #${number} from the owner, kept on his Mac and not posted to the pull request. When you have dealt with a note, call resolve_pull_request_note with its id.`, "", ...lines].join("\n");
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => void listeners.delete(listener);
};

/** The notes for a pull request, read from Fleet when the panel is drawn for it and redrawn as they change. Empty where Fleet serves none. */
export function useJobNotes(handle: NoteHandle | undefined): readonly JobNote[] {
  const key = handle === undefined ? undefined : keyOf(handle);
  useEffect(() => {
    if (handle !== undefined) void loadNotes(handle);
    // The handle is rebuilt each render; the pull request it names is what a read follows.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, handle?.reads]);
  return useSyncExternalStore(subscribe, () => (handle === undefined ? NONE : readNotes(handle)));
}
