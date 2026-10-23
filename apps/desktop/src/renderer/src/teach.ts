// What the owner taught Armada by selecting words in a message: a link rule that draws matching words as links in every
// Session, and a note the agents are given. **Held in this window only, and offered only where a window provides a reader**:
// Fleet's read of the words is not built yet, so a real window has none and draws no Teach. The mock provides one
// (`mock/teach.ts`) and starts every window with nothing taught.

import { useSyncExternalStore } from "react";
import type { TeachReach } from "@armada/components";

/** A rule for words: `pattern` is a regular expression, and `address` writes `{match}` where the matched words go. */
export type TaughtLink = { example: string; pattern: string; address: string };

/** What Fleet's read of the words came to. Either part, or both. */
export type TeachAnswer = { link?: TaughtLink; note?: string };

/** One thing taught, and where it applies: the repository it was taught in, or every one. */
export type Lesson = { id: string; reach: TeachReach; manifestId?: string; repository: string; link?: TaughtLink; note?: string };

export type TeachSource = {
  /** Fleet reads the selected words and what the person said about them. */
  read: (input: { words: string; say: string; reach: TeachReach; repository: string }) => Promise<TeachAnswer>;
};

type Held = { source: TeachSource | undefined; lessons: readonly Lesson[]; preview: Lesson | undefined };

let held: Held = { source: undefined, lessons: [], preview: undefined };
const listeners = new Set<() => void>();
const set = (next: Partial<Held>) => {
  held = { ...held, ...next };
  listeners.forEach((on) => on());
};

/** The window's reader, or none; a new reader starts from nothing taught. */
export function provideTeaching(source: TeachSource | undefined): void {
  set({ source, lessons: [], preview: undefined });
}

/** Asks Fleet to read the words. Throws where the window has no reader, which the Teach act's absence already rules out. */
export function readTeaching(input: Parameters<TeachSource["read"]>[0]): Promise<TeachAnswer> {
  if (held.source === undefined) return Promise.reject(new Error("nothing reads what is taught here"));
  return held.source.read(input);
}

let minted = 0;

export function saveLesson(lesson: Omit<Lesson, "id">): void {
  minted += 1;
  set({ lessons: [...held.lessons, { ...lesson, id: `lesson-${minted}` }], preview: undefined });
}

export function forgetLesson(id: string): void {
  set({ lessons: held.lessons.filter((one) => one.id !== id) });
}

/** The link the panel shows back, drawn in the thread behind it before it is kept. */
export function previewLesson(lesson: Lesson | undefined): void {
  if (held.preview === lesson) return;
  set({ preview: lesson });
}

const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));

/** Whether this window can read what is taught: Teach is offered where it can. */
export function useTeachingOffered(): boolean {
  return useSyncExternalStore(subscribe, () => held.source !== undefined);
}

export function useLessons(): readonly Lesson[] {
  return useSyncExternalStore(subscribe, () => held.lessons);
}

/**
 * The link rules that draw words in a repository's Sessions: those taught there or in every repository, and the one
 * being previewed. In a stable shape, so a thread whose rules did not change is not scanned again.
 */
export function useTaughtLinks(manifestId: string | undefined): readonly { pattern: string; address: string }[] {
  const all = useSyncExternalStore(subscribe, () => held);
  const reaches = (one: Lesson) => one.reach === "all" || one.manifestId === manifestId;
  return [...all.lessons, ...(all.preview === undefined ? [] : [all.preview])].flatMap((one) =>
    one.link === undefined || !reaches(one) ? [] : [{ pattern: one.link.pattern, address: one.link.address }],
  );
}

/** The words of a quote the selection hands over, without the `> ` that leads each line. */
export const wordsOfQuote = (quote: string): string => quote.replace(/^> ?/gm, "").trim();
