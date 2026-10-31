import type { MouseEvent } from "react";
import { ListPlus, MessageCircleQuestion, MessageSquareQuote, Repeat } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { QuickPhrase } from "../ThreadSettings/thread-look";

/** Which kind a reply is, by its glyph alone: the tooltip names it. */
const KIND: Record<"asked" | QuickPhrase["kind"], { Glyph: LucideIcon; said: string }> = {
  asked: { Glyph: MessageCircleQuestion, said: "Asked" },
  learnt: { Glyph: Repeat, said: "Learnt" },
  saved: { Glyph: MessageSquareQuote, said: "Saved" },
};

/** A reply's kind, as a glyph named on hover. */
export function QuickKindMark({ kind }: { kind: "asked" | QuickPhrase["kind"] }) {
  const { Glyph, said } = KIND[kind];
  return (
    <Tooltip label={said}>
      <span className="armada-quick__kind" data-kind={kind} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

export type QuickBarProps = {
  /** Answers to the agent's last question, first. */
  asked: readonly string[];
  /** His own lines, learnt and saved, in the order he keeps them. */
  phrases: readonly QuickPhrase[];
  /** A press sends the line as a message. */
  onSend: (text: string) => void;
  /** A ⇧ press puts the line in the box instead. */
  onDrop: (text: string) => void;
  /** Keeps what the box holds as a saved line. Off while the box is empty. */
  onSave: () => void;
  canSave: boolean;
};

/** The lines the bar offers, in the order ⌘1 to ⌘9 reach them. */
export function quickLines(asked: readonly string[], phrases: readonly QuickPhrase[]): string[] {
  return [...asked, ...phrases.map((one) => one.text)];
}

/**
 * Replies one press away, over the message box: the agent's question's answers, a rule, then his own lines. **Each chip shows
 * the key that sends it**; a ⇧ press puts it in the box to finish instead.
 */
export function QuickBar({ asked, phrases, onSend, onDrop, onSave, canSave }: QuickBarProps) {
  const chips: { key: string; kind: "asked" | QuickPhrase["kind"]; text: string }[] = [
    ...asked.map((text) => ({ key: `asked:${text}`, kind: "asked" as const, text })),
    ...phrases.map((one) => ({ key: one.id, kind: one.kind, text: one.text })),
  ];
  const press = (text: string) => (event: MouseEvent) => (event.shiftKey ? onDrop(text) : onSend(text));
  return (
    <div className="armada-quick" role="group" aria-label="Quick replies">
      {chips.flatMap((one, at) => [
        at === asked.length && at > 0 ? <span key="rule" className="armada-quick__rule" aria-hidden /> : null,
        <button key={one.key} type="button" className="armada-quick__chip" data-kind={one.kind} {...(at < 9 ? { "aria-keyshortcuts": `Meta+${at + 1}` } : {})} onClick={press(one.text)}>
          <QuickKindMark kind={one.kind} />
          <span className="armada-quick__text">{one.text}</span>
          {at < 9 ? <Kbd className="armada-quick__key">⌘{at + 1}</Kbd> : null}
        </button>,
      ])}
      <Tooltip label="Save what is typed as a quick reply">
        <Button type="button" variant="ghost" size="sm" aria-label="Save as quick reply" disabled={!canSave} onClick={onSave}>
          <ListPlus size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </div>
  );
}
