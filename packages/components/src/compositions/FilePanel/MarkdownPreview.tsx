import { useEffect, useRef } from "react";

import { Prose } from "../../primitives/Prose/Prose";
import { lineForTop, topForLine, type Mark } from "./scroll-lock";

/** The blocks as drawn, each at the line it starts on, and the end of the text. */
function marksOf(scroller: HTMLElement, text: string): Mark[] {
  const blocks = [...scroller.querySelectorAll<HTMLElement>("[data-line]")].map((block) => ({ line: Number(block.dataset.line), top: block.offsetTop }));
  return [...blocks, { line: text.split("\n").length + 1, top: scroller.scrollHeight }];
}

/**
 * A markdown file's text, drawn as the markdown it is. Read-only: the edit is made in the file beside it, and this follows.
 * Each block carries the line it starts on, which is what the scroll lock reads.
 */
export function MarkdownPreview({ text, scrollTo, onScroll }: { text: string; scrollTo?: number; onScroll?: (line: number) => void }) {
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
    >
      <Prose text={text} lines />
    </div>
  );
}
