import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { RefObject } from "react";
import { createPortal } from "react-dom";
import { GitFork, Reply } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** What was selected, and the message it was selected in. */
export type ForkPick = { rowId: string; quote: string };

/** Where the selection is: its first line, and the foot of its last. */
type Spot = ForkPick & { left: number; top: number; foot: number };

/** The attribute that makes a block of text forkable, holding the id of the message it belongs to. */
export const FORK_ROW = "data-fork-row";

function pixels(name: string): number {
  return Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name)) || 0;
}

/** The selection as a markdown quote: each line behind `> `, and a blank line after. */
export function quoteOf(words: string): string {
  const lines = words.replace(/\r/g, "").trim().split("\n");
  return `${lines.map((line) => (line.trim() === "" ? ">" : `> ${line.trim()}`)).join("\n")}\n\n`;
}

/** The block of text a node is in, which belongs to one message. */
function blockOf(node: Node): Element | null {
  return (node instanceof Element ? node : node.parentElement)?.closest(`[${FORK_ROW}]`) ?? null;
}

/**
 * The selection, if it lies wholly inside the text of one message under `scope`. **A triple press
 * ends the selection at the start of whatever follows**, so an end outside the block counts where
 * what the block holds from the start says the same words.
 */
function read(scope: HTMLElement): Spot | undefined {
  const selection = window.getSelection();
  if (selection === null || selection.rangeCount === 0 || selection.isCollapsed) return undefined;
  const range = selection.getRangeAt(0).cloneRange();
  const block = blockOf(range.startContainer);
  if (block === null || !scope.contains(block)) return undefined;
  if (blockOf(range.endContainer) !== block) {
    const held = range.cloneRange();
    held.setEnd(block, block.childNodes.length);
    if (held.toString().trim() !== range.toString().trim()) return undefined;
    range.setEnd(block, block.childNodes.length);
  }
  const words = range.toString();
  if (words.trim() === "") return undefined;
  const rects = range.getClientRects?.() ?? [];
  const first = rects[0] ?? range.getBoundingClientRect?.();
  const last = rects[rects.length - 1] ?? first;
  if (first === undefined || last === undefined) return undefined;
  // Out of sight in the thread's own box: scrolled away.
  const box = scope.getBoundingClientRect();
  if (box.height > 0 && (last.bottom < box.top || first.top > box.bottom)) return undefined;
  return { rowId: block.getAttribute(FORK_ROW)!, quote: quoteOf(words), left: first.left + Math.min(first.width, pixels("--space-12") * 2) / 2, top: first.top, foot: last.bottom };
}

/**
 * A small bar above the words selected in a message, with up to two acts: reply to those words, and
 * fork the Session from that message. Each shows only where its handler is given, and the bar
 * where either is. **It reads the selection and never owns it**: a press leaves the selection where it was
 * until the act is taken, and the bar goes when the selection collapses, on Esc, or when the
 * selected words scroll out of `scope`.
 */
export function ForkFromSelection({ scope, onFork, onReply }: { scope: RefObject<HTMLElement | null>; onFork?: (pick: ForkPick) => void; onReply?: (pick: ForkPick) => void }) {
  const [spot, setSpot] = useState<Spot>();
  const bar = useRef<HTMLDivElement>(null);
  const [placed, setPlaced] = useState<{ left: number; top: number }>();

  useEffect(() => {
    const home = scope.current;
    if (home === null) return;
    let dragging = false;
    const look = () => setSpot(read(home));
    // While the pointer is down the selection is still being made.
    const down = (event: MouseEvent) => {
      if (event.button !== 0) return;
      dragging = true;
      setSpot(undefined);
    };
    const up = () => {
      if (!dragging) return;
      dragging = false;
      look();
    };
    const changed = () => {
      if (!dragging) look();
    };
    const key = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || read(home) === undefined) return;
      window.getSelection()?.removeAllRanges();
      setSpot(undefined);
    };
    home.addEventListener("mousedown", down);
    document.addEventListener("mouseup", up);
    document.addEventListener("selectionchange", changed);
    document.addEventListener("keydown", key);
    home.addEventListener("scroll", look, true);
    window.addEventListener("resize", look);
    return () => {
      home.removeEventListener("mousedown", down);
      document.removeEventListener("mouseup", up);
      document.removeEventListener("selectionchange", changed);
      document.removeEventListener("keydown", key);
      home.removeEventListener("scroll", look, true);
      window.removeEventListener("resize", look);
    };
  }, [scope]);

  // Centred over the first line, and kept inside the window; under the last line where there is no room above.
  useLayoutEffect(() => {
    const one = bar.current;
    if (spot === undefined || one === null) return void setPlaced(undefined);
    const { width, height } = one.getBoundingClientRect();
    const gap = pixels("--space-2");
    const edge = pixels("--space-2");
    const above = spot.top - height - gap;
    setPlaced({
      left: Math.min(Math.max(spot.left - width / 2, edge), window.innerWidth - width - edge),
      top: above >= edge ? above : spot.foot + gap,
    });
  }, [spot]);

  if (spot === undefined || (onFork === undefined && onReply === undefined)) return null;
  // Taking an act lets go of the selection, as a press on either does.
  const take = (act: (pick: ForkPick) => void) => {
    act({ rowId: spot.rowId, quote: spot.quote });
    window.getSelection()?.removeAllRanges();
    setSpot(undefined);
  };
  return createPortal(
    <div
      ref={bar}
      className="armada-fork-from-selection"
      role="toolbar"
      aria-label="Selection"
      style={placed === undefined ? { visibility: "hidden", left: 0, top: 0 } : placed}
      // A press must not take the selection with it.
      onMouseDown={(event) => event.preventDefault()}
    >
      {onReply === undefined ? null : (
        <Tooltip label="Reply" beside>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            iconOnly
            aria-label="Reply"
            onClick={() => take(onReply)}
          >
            <Reply size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      {onFork === undefined ? null : (
        <Tooltip label="Fork from here" beside>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            iconOnly
            aria-label="Fork from here"
            onClick={() => take(onFork)}
          >
            <GitFork size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
    </div>,
    document.body,
  );
}
