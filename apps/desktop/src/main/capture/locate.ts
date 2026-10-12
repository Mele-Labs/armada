// What finds the place a Session asked about, in the page of its walk window —
// `docs/practices/capture-window.md`, *A question asked in the window*.
//
// **`layer.ts`'s terms, one ask at a time**: the source travels with the ask, leaves no global
// and no listener, and its answer is the only thing that comes back. It reads the element a
// selector finds and its box, and moves one thing, the page's scroll, as a person's own scroll
// would — never a node, an attribute or a style. The outline is drawn by Bridge's bar over the page.

/**
 * Exactly what this touches in the page, declared because main's project carries no `lib.dom`:
 * the list *is* the review's claim, checked by the compiler.
 */
type PageElement = {
  textContent: string | null;
  getBoundingClientRect: () => { x: number; y: number; width: number; height: number };
  scrollIntoView: (options: { block: "center"; inline: "nearest" }) => void;
};

declare const document: {
  querySelectorAll: (selector: string) => ArrayLike<PageElement>;
};

/**
 * The whole of what runs in the page. **Self-contained, with every value an argument**, because it
 * is stringified. `text` picks the first match that shows it, and the first match stands where none
 * does. `scroll` brings the element to the middle of the viewport, once per question.
 */
export function findsThePlace(selector: string, text: string | null, scroll: boolean): unknown {
  let found: ArrayLike<PageElement>;
  try {
    found = document.querySelectorAll(selector);
  } catch {
    // A selector the page's engine will not parse finds nothing; it is not the page's fault.
    return null;
  }
  const all = Array.from<PageElement>(found);
  const flat = (said: string | null): string => (said ?? "").replace(/\s+/g, " ").trim().toLowerCase();
  const wanted = flat(text);
  const element = (wanted === "" ? undefined : all.find((one) => flat(one.textContent).includes(wanted))) ?? all[0];
  if (element === undefined) return null;
  if (scroll) element.scrollIntoView({ block: "center", inline: "nearest" });
  const rect = element.getBoundingClientRect();
  return {
    box: { x: Math.round(rect.x), y: Math.round(rect.y), width: Math.round(rect.width), height: Math.round(rect.height) },
  };
}

/** The source of one ask, as main hands it to `executeJavaScript`: an expression whose value is the answer. */
export function locateSource(selector: string, text: string | null, scroll: boolean): string {
  const args = [selector, text, scroll].map((value) => JSON.stringify(value)).join(", ");
  return `(${findsThePlace.toString()})(${args})`;
}
