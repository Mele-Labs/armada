// Where a note stands in a page drawn in the layer: the words it quoted, the row it named. Read from the page as it is drawn, so a note
// finds its place again after the page is redrawn, and in a revision that still holds the words.

import { onOneLine, type DocumentTarget, type NoteAbout, type NoteAnchor } from "@armada/components";

import { rowText, sectionOf } from "./document-page";

/** Text the reader did not write into the page: a struck word, a sketch's labels, the numbers that mark a note. */
const NOT_TEXT = "del, .armada-doc__sketch, .armada-doc__anchor";

/** The range of the page's text that reads as `quote`, whitespace and quote marks set as a note's heading sets them. */
export function rangeOf(page: HTMLElement, quote: string): Range | undefined {
  const want = onOneLine(quote);
  if (want === "") return undefined;
  const walker = document.createTreeWalker(page, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => (node.parentElement?.closest(NOT_TEXT) == null ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT),
  });
  let flat = "";
  const at: { node: Text; offset: number }[] = [];
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const text = node as Text;
    for (let offset = 0; offset < text.data.length; offset += 1) {
      const ch = text.data.charAt(offset);
      if (/\s/.test(ch)) {
        if (flat === "" || flat.endsWith(" ")) continue;
        flat += " ";
      } else flat += ch === '"' ? "'" : ch;
      at.push({ node: text, offset });
    }
  }
  const start = flat.indexOf(want);
  if (start < 0) return undefined;
  const first = at[start];
  const last = at[start + want.length - 1];
  if (first === undefined || last === undefined) return undefined;
  const range = document.createRange();
  range.setStart(first.node, first.offset);
  range.setEnd(last.node, last.offset + 1);
  return range;
}

/** The row of a table whose cells read as `row`. */
export function rowOf(page: HTMLElement, row: string): HTMLElement | undefined {
  const want = onOneLine(row);
  return [...page.querySelectorAll<HTMLElement>("tbody tr")].find((one) => rowText(one) === want);
}

/** The element a note's target is drawn at, for scrolling to it. */
export function placeOf(page: HTMLElement, target: DocumentTarget): Element | undefined {
  if (target.kind === "row") return rowOf(page, target.row);
  if (target.kind === "part") return [...page.querySelectorAll(".armada-doc__sketch")].find((one) => one.getAttribute("data-title") === target.sketch);
  const range = rangeOf(page, target.quote);
  const node = range?.startContainer;
  return node === undefined ? undefined : node instanceof Element ? node : (node.parentElement ?? undefined);
}

export const anchorOf = (rect: DOMRect): NoteAnchor => ({ left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom });

export type Pointed = { about: Extract<NoteAbout, { kind: "document" }>; anchor: NoteAnchor };

/** What a right press or the menu key points at in the page: the words selected, else the table row it is on. */
export function pointedAt(page: HTMLElement, target: EventTarget | null, title: string, revision: number): Pointed | undefined {
  const selection = window.getSelection();
  if (selection !== null && selection.rangeCount > 0 && !selection.isCollapsed && page.contains(selection.anchorNode) && page.contains(selection.focusNode)) {
    const range = selection.getRangeAt(0);
    const quote = onOneLine(selection.toString());
    // A press somewhere else than on the words is about where it landed.
    if (target instanceof Node && !range.intersectsNode(target)) return rowAt(page, target, title, revision);
    const inside = range.startContainer instanceof Element ? range.startContainer : range.startContainer.parentElement;
    if (quote !== "" && inside !== null && inside.closest(".armada-doc__sketch") === null) {
      const place = { page: title, revision, section: sectionOf(page, inside, title) };
      return { about: { kind: "document", place, target: { kind: "words", quote } }, anchor: anchorOf(range.getBoundingClientRect()) };
    }
  }
  return rowAt(page, target, title, revision);
}

function rowAt(page: HTMLElement, target: EventTarget | null, title: string, revision: number): Pointed | undefined {
  const row = target instanceof Element ? target.closest<HTMLElement>("tbody tr") : null;
  if (row === null || !page.contains(row)) return undefined;
  const place = { page: title, revision, section: sectionOf(page, row, title) };
  const table = onOneLine(row.closest("table")?.querySelector("caption")?.textContent ?? "Table");
  return { about: { kind: "document", place, target: { kind: "row", table, row: rowText(row) } }, anchor: anchorOf(row.getBoundingClientRect()) };
}
