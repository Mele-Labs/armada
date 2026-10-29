// What Bridge's capture window runs inside a page to play a walk's steps — `docs/practices/capture-window.md`,
// *A walk in the page*. **Built into a string** by `electron.vite.config.ts` and evaluated by main in
// an isolated world of its own, so the page's scripts see none of it: not the global it defines, and
// not the element it holds. It reads the page, and moves one thing, its scroll, as a person's own would.
//
// It acts on nothing. A press, a key and a drag are real input events that main sends, aimed at the
// box this answers with; the page hears them as it hears a person.

import { arriveAt } from "./walk-find";
import type { Seeking } from "./walk-find";
import { snapshotOf } from "./job-browser/snapshot";

/** A box in the page's viewport, in CSS pixels. */
export type Seen = { x: number; y: number; width: number; height: number };

/** What the held element is, for the keys that must reach it. */
export type Kind = "field" | "select" | "other";

let held: HTMLElement | null = null;

/** The part of the element that is on screen, rounded, or null where none of it is. */
function onScreen(element: HTMLElement): Seen | null {
  const box = element.getBoundingClientRect();
  const left = Math.max(0, box.left);
  const top = Math.max(0, box.top);
  const right = Math.min(window.innerWidth, box.right);
  const bottom = Math.min(window.innerHeight, box.bottom);
  if (right <= left || bottom <= top) return null;
  return { x: Math.round(left), y: Math.round(top), width: Math.round(right - left), height: Math.round(bottom - top) };
}

export const finder = {
  /** Look for the target, scroll it into view, and wait for it to hold still. It is held for what follows. */
  async arrive(target: Seeking, patience: number): Promise<Seen | null> {
    held = await arriveAt(target, patience);
    return held === null ? null : onScreen(held);
  },

  /** Where the held element is now. Null: it left the page, or none is held. */
  box(): Seen | null {
    return held === null || !held.isConnected ? null : onScreen(held);
  },

  /** Give the held element the keyboard, without pressing it, and say what it is. */
  focus(): Kind | null {
    if (held === null || !held.isConnected) return null;
    held.focus({ preventScroll: true });
    if (held instanceof HTMLSelectElement) return "select";
    return held instanceof HTMLInputElement || held instanceof HTMLTextAreaElement || held.isContentEditable ? "field" : "other";
  },

  /** Where `words` start and end inside the held element, for a pointer to drag across. Null: not there. */
  words(words: string): { from: { x: number; y: number }; to: { x: number; y: number } } | null {
    if (held === null || !held.isConnected) return null;
    const nodes = document.createTreeWalker(held, NodeFilter.SHOW_TEXT);
    for (let node = nodes.nextNode(); node !== null; node = nodes.nextNode()) {
      const at = (node as Text).data.indexOf(words);
      if (at < 0) continue;
      const range = document.createRange();
      range.setStart(node, at);
      range.setEnd(node, at + words.length);
      const rects = [...range.getClientRects()].filter((one) => one.width > 0 && one.height > 0);
      const first = rects[0];
      const last = rects[rects.length - 1];
      if (first === undefined || last === undefined) return null;
      return {
        from: { x: Math.round(first.left + 1), y: Math.round(first.top + first.height / 2) },
        to: { x: Math.round(last.right - 1), y: Math.round(last.top + last.height / 2) },
      };
    }
    return null;
  },

  /** Let go of the element. */
  release(): void {
    held = null;
  },

  /** The page's accessibility tree as text, for a Job browser's agent, cut at `most` characters. */
  snapshot(most: number): string {
    return document.body === null ? "" : snapshotOf(document.body, most);
  },
};
