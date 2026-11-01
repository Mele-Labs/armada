import { createContext, useContext, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { Check, ThumbsDown, ThumbsUp } from "lucide-react";

import { Button } from "../Button/Button";
import { Tooltip } from "../Tooltip/Tooltip";

/** What a press on a line says: this one, yes to it, no to it. */
export type PickKind = "pick" | "yes" | "no";

/**
 * Where a line of an agent's words can be answered. Provided by a thread around an agent's message; without one
 * no line is hoverable and `Prose` draws as it always has. `marked` is what the person already said to a line.
 */
export type ProsePicker = {
  pick: (text: string, kind: PickKind) => void;
  marked: (text: string) => PickKind | undefined;
};
export const ProsePick = createContext<ProsePicker | null>(null);

/** Set inside a pickable list item, whose paragraphs are part of its line and not lines of their own. */
export const InPickLine = createContext(false);

const ACTS: readonly { kind: PickKind; label: string; Glyph: typeof Check }[] = [
  { kind: "pick", label: "Pick this", Glyph: Check },
  { kind: "yes", label: "Yes to this", Glyph: ThumbsUp },
  { kind: "no", label: "No to this", Glyph: ThumbsDown },
];

const MARKS: Record<PickKind, { label: string; Glyph: typeof Check }> = {
  pick: { label: "Picked", Glyph: Check },
  yes: { label: "Yes", Glyph: ThumbsUp },
  no: { label: "No", Glyph: ThumbsDown },
};

/** The hast node `react-markdown` hands a component, as far as the line's words are read from it. */
export type LineNode = { type: string; value?: string; tagName?: string; children?: LineNode[] };

/** The words of a line as plain text: formatting dropped, code kept, and a nested list left to its own lines. */
export function lineText(node: LineNode | undefined): string {
  const out: string[] = [];
  const walk = (one: LineNode, top: boolean) => {
    if (one.type === "text") out.push(one.value ?? "");
    if (one.type !== "element" && one.type !== "root") return;
    if (!top && (one.tagName === "ul" || one.tagName === "ol" || one.tagName === "input")) return;
    if (one.tagName === "br") out.push(" ");
    one.children?.forEach((child) => walk(child, false));
    if (one.tagName === "p") out.push(" ");
  };
  if (node !== undefined) walk(node, true);
  return out.join("").replace(/\s+/g, " ").trim();
}

/** How far past the line and its acts the pointer may stray before they go: leaving has to be clear, not a slip. */
const STRAY = 24;
/** How long the pointer must rest on another line before it takes the acts: crossing one on the way to the acts is not choosing it. */
const SETTLE_MS = 250;
/** How long the acts outlast the pointer leaving, so a quick overshoot does not lose them. */
const LINGER_MS = 300;

/** The one hot line: how to cool it. A line the pointer settles on cools the one before. */
let coolHot: (() => void) | null = null;
/** A line waiting to take the acts once the pointer has settled on it; reaching the hot line's acts calls it off. */
let settling: ReturnType<typeof setTimeout> | undefined;

/** Arrow keys walk the lines of the message the focus is in. */
function move(from: HTMLElement, by: 1 | -1): void {
  const message = from.closest(".armada-prose");
  if (message === null) return;
  const lines = [...message.querySelectorAll<HTMLElement>("[data-pick]")];
  lines[lines.indexOf(from) + by]?.focus();
}

/**
 * A list item or a paragraph of an agent's message that can be answered: a wash on hover or focus, and Pick, Yes and No
 * over its right end. Enter picks, Y says yes, N says no, the arrows move between lines. What was answered keeps a quiet mark.
 */
export function PickLine({ as: Tag, className, text, children, ...rest }: { as: "li" | "p"; className: string; text: string; children?: ReactNode; [data: `data-${string}`]: unknown }) {
  const picker = useContext(ProsePick);
  // Whether this line holds the acts. Read from the pointer's own events, so the nearest line wins over a list inside it.
  const [hot, setHot] = useState(false);
  const line = useRef<HTMLElement | null>(null);
  const acts = useRef<HTMLSpanElement | null>(null);
  // Whether the pointer is over this line now, for a line still settling.
  const over = useRef(false);
  // While hot, the acts stay until the pointer is clearly away from the line and from them, and a moment after.
  useEffect(() => {
    if (!hot) return;
    let leaving: ReturnType<typeof setTimeout> | undefined;
    const cool = () => {
      clearTimeout(leaving);
      setHot(false);
    };
    coolHot = cool;
    const within = (one: Element | null, by: number, event: PointerEvent) => {
      if (one === null) return false;
      const box = one.getBoundingClientRect();
      return event.clientX >= box.left - by && event.clientX <= box.right + by && event.clientY >= box.top - by && event.clientY <= box.bottom + by;
    };
    const onMove = (event: PointerEvent) => {
      if (within(acts.current, STRAY, event)) clearTimeout(settling);
      if (within(line.current, STRAY, event) || within(acts.current, STRAY, event)) {
        clearTimeout(leaving);
        leaving = undefined;
      } else if (leaving === undefined) {
        leaving = setTimeout(cool, LINGER_MS);
      }
    };
    document.addEventListener("pointermove", onMove);
    return () => {
      clearTimeout(leaving);
      document.removeEventListener("pointermove", onMove);
      if (coolHot === cool) coolHot = null;
    };
  }, [hot]);
  // `rest` is the `data-line` a source view puts on a block.
  if (picker === null || text === "") return <Tag className={className} {...rest}>{children}</Tag>;
  const marked = picker.marked(text);
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.target !== event.currentTarget || event.metaKey || event.ctrlKey || event.altKey) return;
    const kind = event.key === "Enter" ? "pick" : event.key === "y" ? "yes" : event.key === "n" ? "no" : undefined;
    if (kind !== undefined) {
      event.preventDefault();
      picker.pick(text, kind);
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      move(event.currentTarget, event.key === "ArrowDown" ? 1 : -1);
    }
  };
  const Mark = marked === undefined ? null : MARKS[marked];
  return (
    <Tag ref={(node: HTMLElement | null) => { line.current = node; }} className={className} {...rest} data-pick="" data-hot={hot || undefined} data-marked={marked} tabIndex={0}
      onPointerOver={(event) => {
        const mine = (event.target as Element).closest("[data-pick]") === event.currentTarget;
        over.current = mine;
        if (hot || !mine || acts.current?.contains(event.target as Node)) return;
        const take = () => {
          coolHot?.();
          setHot(true);
        };
        clearTimeout(settling);
        // Nothing hot: take the acts at once. Another line hot: only once the pointer rests here.
        if (coolHot === null) take();
        else settling = setTimeout(() => over.current && take(), SETTLE_MS);
      }}
      onPointerOut={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) over.current = false;
      }} aria-keyshortcuts="Enter Y N" onKeyDown={onKeyDown}>
      <InPickLine.Provider value>{children}</InPickLine.Provider>
      {Mark === null ? null : (
        <Tooltip label={Mark.label}>
          <span className="armada-prose__pick-mark" role="img" aria-label={Mark.label}>
            <Mark.Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      )}
      <span ref={acts} className="armada-prose__pick-acts" role="group" aria-label={`Answer: ${text}`}>
        {ACTS.map(({ kind, label, Glyph }) => (
          <Tooltip key={kind} label={label}>
            <Button variant="ghost" size="sm" iconOnly aria-label={label} tabIndex={-1} onClick={(event) => {
              picker.pick(text, kind);
              setHot(false);
              // A press leaves focus on the button, which would hold the acts up after the pointer has gone.
              event.currentTarget.blur();
            }}>
              <Glyph size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        ))}
      </span>
    </Tag>
  );
}
