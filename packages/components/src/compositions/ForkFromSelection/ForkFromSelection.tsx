import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { RefObject } from "react";
import { createPortal } from "react-dom";
import { GitFork, GraduationCap, Reply } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import type { DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
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

/** The words as the right-click menu holds them: where it opens, and the pick it was opened on, kept because opening it may take the selection. */
type Opened = { x: number; y: number; pick: ForkPick };

const TEACH = { id: "teach", label: "Teach", icon: GraduationCap } as const;
const REPLY = { id: "reply", label: "Reply", icon: Reply } as const;
const FORK = { id: "fork", label: "Fork from here", icon: GitFork } as const;
/** The menu lists Teach first, the act the person came for; the bar keeps Reply and Fork where they were and adds Teach after them. */
const IN_MENU = [TEACH, REPLY, FORK];
const ON_BAR = [REPLY, FORK, TEACH];

/**
 * A small bar above the words selected in a message, with up to three acts: reply to those words, fork the
 * Session from that message, and teach Armada what the words are. Each shows only where its handler is given, and the bar
 * where any is. **A right press on the selected words opens the same acts as a menu**, and so do the ContextMenu key and
 * Shift+F10 while words are selected; a right press anywhere else falls through to whatever the row offers. **It reads the selection and never owns it**: a press leaves the selection where it was
 * until the act is taken, and the bar goes when the selection collapses, on Esc, or when the
 * selected words scroll out of `scope`.
 */
export function ForkFromSelection({
  scope,
  onFork,
  onReply,
  onTeach,
}: {
  scope: RefObject<HTMLElement | null>;
  onFork?: (pick: ForkPick) => void;
  onReply?: (pick: ForkPick) => void;
  onTeach?: (pick: ForkPick) => void;
}) {
  const [spot, setSpot] = useState<Spot>();
  const [menu, setMenu] = useState<Opened>();
  const acts = { teach: onTeach, reply: onReply, fork: onFork };
  const onBar = ON_BAR.filter((one) => acts[one.id] !== undefined);
  const live = useRef({ offered: onBar.length, menu: false });
  live.current = { offered: onBar.length, menu: menu !== undefined };
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
    // The menu from the keyboard opens under the selected words. **In the capture phase and stopped there**, as a right press is.
    const menuKey = (event: KeyboardEvent) => {
      if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
      const there = live.current.offered === 0 ? undefined : read(home);
      if (there === undefined) return;
      event.preventDefault();
      event.stopPropagation();
      setMenu({ x: there.left, y: there.foot, pick: { rowId: there.rowId, quote: there.quote } });
    };
    const key = (event: KeyboardEvent) => {
      // With the menu open, Esc closes the menu and leaves the words selected.
      if (event.key !== "Escape" || live.current.menu || read(home) === undefined) return;
      window.getSelection()?.removeAllRanges();
      setSpot(undefined);
    };
    // A right press on the selected words. **In the capture phase and stopped there**, so a row that has a menu of its
    // own (Fleet's notice) keeps it for every press that is not on selected words.
    const context = (event: MouseEvent) => {
      const there = live.current.offered === 0 ? undefined : read(home);
      if (there === undefined || !(event.target instanceof Node) || blockOf(event.target)?.getAttribute(FORK_ROW) !== there.rowId) return;
      const range = window.getSelection()?.getRangeAt(0);
      const box = range?.getBoundingClientRect();
      if (box === undefined || event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom) return;
      event.preventDefault();
      event.stopPropagation();
      setMenu({ x: event.clientX, y: event.clientY, pick: { rowId: there.rowId, quote: there.quote } });
    };
    home.addEventListener("contextmenu", context, true);
    home.addEventListener("mousedown", down);
    document.addEventListener("mouseup", up);
    document.addEventListener("selectionchange", changed);
    document.addEventListener("keydown", menuKey, true);
    document.addEventListener("keydown", key);
    home.addEventListener("scroll", look, true);
    window.addEventListener("resize", look);
    return () => {
      home.removeEventListener("contextmenu", context, true);
      home.removeEventListener("mousedown", down);
      document.removeEventListener("mouseup", up);
      document.removeEventListener("selectionchange", changed);
      document.removeEventListener("keydown", menuKey, true);
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

  // Taking an act lets go of the selection, as a press on any of them does.
  const take = (act: (pick: ForkPick) => void, pick: ForkPick) => {
    act(pick);
    window.getSelection()?.removeAllRanges();
    setSpot(undefined);
    setMenu(undefined);
  };
  if (menu !== undefined) {
    return (
      <DropdownMenu
        triggerLabel="Selection"
        at={{ x: menu.x, y: menu.y }}
        open
        entries={IN_MENU.filter((one) => acts[one.id] !== undefined).map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.label, icon: one.icon }))}
        onOpenChange={(open) => {
          if (!open) setMenu(undefined);
        }}
        onSelect={(chosen) => {
          const act = acts[chosen as keyof typeof acts];
          if (act !== undefined) take(act, menu.pick);
        }}
      />
    );
  }
  if (spot === undefined || onBar.length === 0) return null;
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
      {onBar.map(({ id, label, icon: Glyph }) => (
        <Tooltip key={id} label={label} beside>
          <Button type="button" variant="ghost" size="sm" iconOnly aria-label={label} onClick={() => take(acts[id]!, { rowId: spot.rowId, quote: spot.quote })}>
            <Glyph size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      ))}
    </div>,
    document.body,
  );
}
