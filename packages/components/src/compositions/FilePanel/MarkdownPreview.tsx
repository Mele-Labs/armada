import { useEffect, useRef } from "react";

import { Prose } from "../../primitives/Prose/Prose";
import type { CodeLines, CodeNoteMark } from "./CodeEditor";
import { lineForTop, topForLine, type Mark } from "./scroll-lock";

/** The blocks as drawn, each at the line it starts on, and the end of the text. */
function marksOf(scroller: HTMLElement, text: string): Mark[] {
  const blocks = [...scroller.querySelectorAll<HTMLElement>("[data-line]")].map((block) => ({ line: Number(block.dataset.line), top: block.offsetTop }));
  return [...blocks, { line: text.split("\n").length + 1, top: scroller.scrollHeight }];
}

/** A line with markdown's own marks taken off, so words as drawn can be found in it. */
const plain = (line: string) => line.replace(/[*_`#>[\]()~|]/g, "").replace(/\s+/g, " ").trim().toLowerCase();

/** How much of each end of a quote is looked for: short enough not to cross a line the source broke and the preview did not. */
const NEEDLE = 16;

/**
 * The source lines words selected in the preview came from: the line the first words stand on, from the block they are in,
 * to the line the last words do. Where the words are not found, the block's own line.
 */
function linesFor(text: string, quote: string, near: number): CodeLines {
  const source = text.split("\n");
  const flat = source.map(plain);
  const parts = quote.split("\n").map(plain).filter((one) => one !== "");
  const find = (needle: string, start: number): number | undefined => {
    for (let at = Math.max(0, start - 1); at < flat.length; at += 1) if (flat[at]!.includes(needle)) return at + 1;
    return undefined;
  };
  const first = parts[0];
  const last = parts[parts.length - 1];
  const from = first === undefined ? near : (find(first.slice(0, NEEDLE), near) ?? find(first.slice(0, NEEDLE), 1) ?? near);
  const to = last === undefined ? from : Math.max(from, find(last.slice(-NEEDLE), from) ?? from);
  return { from, to, quote: source.slice(from - 1, to).join("\n") };
}

/**
 * A markdown file's text, drawn as the markdown it is. Read-only: the edit is made in the file beside it, and this follows.
 * Each block carries the line it starts on, which is what the scroll lock reads, and what a note on selected words is placed by.
 */
export function MarkdownPreview({
  text,
  scrollTo,
  onScroll,
  notes,
  onNote,
}: {
  text: string;
  scrollTo?: number;
  onScroll?: (line: number) => void;
  /** Held notes: the blocks holding their lines are marked with their numbers. */
  notes?: readonly CodeNoteMark[];
  /** A right press, or the menu key or Shift F10, on selected words: where, the source lines they came from, and where the words are. */
  onNote?: (at: { x: number; y: number }, lines: CodeLines, drawn: { left: number; top: number; right: number; bottom: number } | undefined) => void;
}) {
  const scroller = useRef<HTMLDivElement>(null);
  // Where `scrollTo` last put the view: the scroll that move makes is its own, and is not a person's.
  const put = useRef<number | null>(null);

  useEffect(() => {
    const el = scroller.current;
    if (el === null || scrollTo === undefined) return;
    const top = topForLine(marksOf(el, text), scrollTo);
    if (Math.abs(top - el.scrollTop) < 1) return;
    put.current = top;
    el.scrollTop = top;
    // Moves where the text is read from, not when the text changes: typing must not move the view.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scrollTo]);

  // Each block a held note's lines fall in is marked, with the note's numbers.
  const noted = JSON.stringify(notes ?? []);
  useEffect(() => {
    const el = scroller.current;
    if (el === null) return;
    const marks = JSON.parse(noted) as CodeNoteMark[];
    const blocks = [...el.querySelectorAll<HTMLElement>("[data-line]")];
    blocks.forEach((block, at) => {
      const from = Number(block.dataset.line);
      const next = blocks[at + 1];
      const to = next === undefined ? Number.POSITIVE_INFINITY : Number(next.dataset.line) - 1;
      const here = marks.filter((one) => one.from <= to && one.to >= from);
      if (here.length === 0) {
        delete block.dataset.noted;
        delete block.dataset.note;
        delete block.dataset.writing;
        return;
      }
      block.dataset.noted = "";
      block.dataset.note = here.map((one) => String(one.n)).join(" ");
      if (here.some((one) => one.writing)) block.dataset.writing = "";
      else delete block.dataset.writing;
    });
  }, [noted, text]);

  const noteAt = (at: { x: number; y: number }) => {
    const chosen = window.getSelection();
    const words = chosen?.toString() ?? "";
    if (onNote === undefined || chosen === null || words.trim() === "" || chosen.anchorNode === null || !scroller.current?.contains(chosen.anchorNode)) return false;
    const start = chosen.getRangeAt(0).startContainer;
    const block = (start instanceof Element ? start : start.parentElement)?.closest<HTMLElement>("[data-line]");
    const drawn = chosen.getRangeAt(0).getBoundingClientRect();
    onNote(at, linesFor(text, words, Number(block?.dataset.line ?? 1)), { left: drawn.left, top: drawn.top, right: drawn.right, bottom: drawn.bottom });
    return true;
  };

  return (
    <div
      ref={scroller}
      className="armada-markdown-preview"
      role="region"
      aria-label="Rendered markdown"
      tabIndex={0}
      onScroll={(event) => {
        const el = event.currentTarget;
        if (put.current !== null && Math.abs(el.scrollTop - put.current) < 2) {
          put.current = null;
          return;
        }
        put.current = null;
        onScroll?.(lineForTop(marksOf(el, text), el.scrollTop));
      }}
      onContextMenu={(event) => {
        if (noteAt({ x: event.clientX, y: event.clientY })) event.preventDefault();
      }}
      onKeyDown={(event) => {
        if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
        const chosen = window.getSelection();
        const box = chosen === null || chosen.rangeCount === 0 ? undefined : chosen.getRangeAt(0).getBoundingClientRect();
        if (box !== undefined && noteAt({ x: box.left, y: box.bottom })) event.preventDefault();
      }}
    >
      <Prose text={text} lines />
    </div>
  );
}
