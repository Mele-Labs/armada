// What the native Job browser runs in a page to read the part of it at a point, for a note: **Classic's own layer**
// (`readsThePage` in `layer.ts`, the one `askSource` hands the walk window) with the part's role and accessible name beside
// its answer, which is what the note card says (`fast.com › button · Google Search`). Built into a script by
// `scripts/job-browser-page.mjs` and run in a content world of Bridge's own, which shares the page's DOM and nothing else:
// React's expando is not visible there, so a page's component names are not read, and the selector, nearby names, stamp and
// marked state are.
//
// It reads the page and writes nothing to it, as the layer does.

import { computeAccessibleName, getRole } from "dom-accessibility-api";
import { ASK_BOUNDS, readsThePage } from "./layer";

/** Roles that say nothing of their own: a part inside one is named by the nearest part that does say something. */
const QUIET = new Set([
  "generic",
  "none",
  "presentation",
  "paragraph",
  "group",
  "list",
  "listitem",
  "row",
  "rowgroup",
]);

/** How far up the words look for a part that has a role and a name. */
const MOST_CLIMB = 6;
const MOST_NAME = 120;

type Part = {
  parentElement: Part | null;
  closest: (selector: string) => Part | null;
  tagName: string;
};

declare const document: {
  elementFromPoint: (x: number, y: number) => Part | null;
};

const squeezed = (words: string): string => words.replace(/\s+/g, " ").trim();

/**
 * What the part at a point is called: the nearest part, itself or one it is drawn in, that has a role of its own and a name,
 * else its own role with no name.
 */
function wordsOf(
  x: number,
  y: number,
): {
  role: string;
  name: string;
} | null {
  const found = document.elementFromPoint(x, y);
  if (found === null) return null;
  const own = found.closest("svg") ?? found;
  let at: Part | null = own;
  for (let step = 0; at !== null && step <= MOST_CLIMB; step += 1) {
    const role = getRole(at as never);
    if (role !== null && !QUIET.has(role)) {
      const name = squeezed(computeAccessibleName(at as never)).slice(0, MOST_NAME);
      if (name !== "")
        return {
          role,
          name,
        };
    }
    at = at.parentElement;
  }
  const role = getRole(own as never);
  return {
    role: role === null || QUIET.has(role) ? "" : role,
    name: "",
  };
}

export const reader = {
  /** `readsThePage`'s answer at a point of the viewport, with `role` and `name`. Null: nothing is there. */
  ask(x: number, y: number, take: boolean): unknown {
    const answer = readsThePage(x, y, take, ASK_BOUNDS);
    if (typeof answer !== "object" || answer === null) return null;
    return {
      ...answer,
      ...wordsOf(x, y),
    };
  },
};
