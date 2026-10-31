import { createContext, useContext, useState, type KeyboardEvent, type ReactNode } from "react";
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
  // Whether the pointer is over this line and not over a list inside it: read from the pointer's own events, so the nearest line wins.
  const [hot, setHot] = useState(false);
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
    <Tag className={className} {...rest} data-pick="" data-hot={hot || undefined} data-marked={marked} tabIndex={0}
      onPointerOver={(event) => setHot((event.target as Element).closest("[data-pick]") === event.currentTarget)}
      onPointerOut={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setHot(false);
      }} aria-keyshortcuts="Enter Y N" onKeyDown={onKeyDown}>
      <InPickLine.Provider value>{children}</InPickLine.Provider>
      {Mark === null ? null : (
        <Tooltip label={Mark.label}>
          <span className="armada-prose__pick-mark" role="img" aria-label={Mark.label}>
            <Mark.Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      )}
      <span className="armada-prose__pick-acts" role="group" aria-label={`Answer: ${text}`}>
        {ACTS.map(({ kind, label, Glyph }) => (
          <Tooltip key={kind} label={label}>
            <Button variant="ghost" size="sm" iconOnly aria-label={label} tabIndex={-1} onClick={() => picker.pick(text, kind)}>
              <Glyph size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        ))}
      </span>
    </Tag>
  );
}
