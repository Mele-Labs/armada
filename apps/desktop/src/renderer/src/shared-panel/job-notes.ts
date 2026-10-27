// Notes the owner leaves for the Job that holds a pull request, on lines of its diff. **They are kept on this Mac in
// `localStorage` and never reach the forge**: nothing here imports a write to it. A note leaves only when he sends the
// unsent ones to the Job, as one message.

import { useSyncExternalStore } from "react";
import type { DiffSide } from "@armada/protocol";

export type JobNote = { id: string; path: string; line: number; side: DiffSide; body: string; sent: boolean };

const PREFIX = "armada:job-notes:";

const listeners = new Set<() => void>();
const storeKey = (key: string): string => `${PREFIX}${key}`;

/** What is kept for a pull request, as `repository#number`. A value that is not a list of notes reads as none. */
export function readNotes(key: string): JobNote[] {
  try {
    const kept: unknown = JSON.parse(window.localStorage.getItem(storeKey(key)) ?? "[]");
    return Array.isArray(kept) ? (kept as JobNote[]) : [];
  } catch {
    return [];
  }
}

function keep(key: string, notes: JobNote[]): JobNote[] {
  try {
    if (notes.length === 0) window.localStorage.removeItem(storeKey(key));
    else window.localStorage.setItem(storeKey(key), JSON.stringify(notes));
  } catch {
    // A full or blocked store keeps nothing, and the notes drawn are the ones read back.
  }
  cache.delete(key);
  listeners.forEach((one) => one());
  return readNotes(key);
}

/** A blank body is not kept. Answers every note kept for the pull request. */
export function addNote(key: string, note: Omit<JobNote, "id" | "sent">): JobNote[] {
  const body = note.body.trim();
  if (body === "") return readNotes(key);
  return keep(key, [...readNotes(key), { ...note, body, id: `n${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`, sent: false }]);
}

export const editNote = (key: string, id: string, body: string): JobNote[] =>
  body.trim() === "" ? readNotes(key) : keep(key, readNotes(key).map((one) => (one.id === id ? { ...one, body: body.trim() } : one)));

export const removeNote = (key: string, id: string): JobNote[] => keep(key, readNotes(key).filter((one) => one.id !== id));

export const markSent = (key: string, ids: readonly string[]): JobNote[] => keep(key, readNotes(key).map((one) => (ids.includes(one.id) ? { ...one, sent: true } : one)));

const SIDE_SAID = " (the removed line)";

/** What the Job reads: each note numbered, with the file and line the forge would name it by. */
export function messageOf(number: number, notes: readonly JobNote[]): string {
  const lines = notes.map((one, at) => `${at + 1}. ${one.path}:${one.line}${one.side === "left" ? SIDE_SAID : ""}: ${one.body}`);
  return [`Notes on #${number} from the owner, kept on his Mac and not posted to the pull request`, "", ...lines].join("\n");
}

// One array per key until it changes, so `useSyncExternalStore` sees the same snapshot between renders.
const cache = new Map<string, JobNote[]>();
const snapshot = (key: string): JobNote[] => {
  const held = cache.get(key);
  if (held !== undefined) return held;
  const read = readNotes(key);
  cache.set(key, read);
  return read;
};

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => void listeners.delete(listener);
};

/** The notes for a pull request, redrawn as they change. */
export function useJobNotes(key: string): JobNote[] {
  return useSyncExternalStore(subscribe, () => snapshot(key));
}
