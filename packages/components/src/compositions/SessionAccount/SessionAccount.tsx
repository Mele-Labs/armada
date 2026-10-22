import { useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { ChevronDown, ChevronRight, ChevronUp } from "lucide-react";

import { Card } from "../../primitives/Card/Card";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { TASK_GLYPH, taskMarkSaid } from "../TaskMark/TaskMark";
import { useEscapeFold } from "../NeedsYouPanel/useEscapeFold";
import { useScrimFold } from "../NeedsYouPanel/useScrimFold";

/**
 * The running account of a turn, in his words: what is done for his message, what it is on now and what is next. `now` and `next` are
 * absent once the turn has ended. The lines are the agent's own; nothing here adds a word to them.
 */
export type TurnAccount = { done: readonly string[]; now?: string; next?: string };

/** The page or sketch being built, as the host draws it: `view` is the frame, `title` names it. */
export type TurnLive = { title: string; view: ReactNode };

type Line = "done" | "now" | "next";

const TASK: Record<Line, "done" | "working" | "open"> = { done: "done", now: "working", next: "open" };
const SAID: Record<Line, string> = { done: taskMarkSaid("done"), now: taskMarkSaid("working"), next: "Next" };

/** A line's registry mark, named on hover. Only the line being worked breathes. */
function LineMark({ line }: { line: Line }) {
  const Glyph = TASK_GLYPH[TASK[line]];
  return (
    <Tooltip label={SAID[line]}>
      <span className="armada-session-mark armada-task-mark" data-state={TASK[line]} data-pulsing={line === "now" || undefined} role="img" aria-label={SAID[line]}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The done lines, then the one it is on, then the one after, each led by its mark. */
export function TurnAccountList({ lines }: { lines: TurnAccount }) {
  const rows: { line: Line; text: string }[] = [
    ...lines.done.map((text) => ({ line: "done" as const, text })),
    ...(lines.now === undefined ? [] : [{ line: "now" as const, text: lines.now }]),
    ...(lines.next === undefined ? [] : [{ line: "next" as const, text: lines.next }]),
  ];
  return (
    <ol className="armada-account__lines" aria-label="Account">
      {rows.map((one, at) => (
        <li key={`${one.line}:${at}`} className="armada-account__line" data-line={one.line}>
          <LineMark line={one.line} />
          <span className="armada-account__text">{one.text}</span>
        </li>
      ))}
    </ol>
  );
}

/** The live view: the frame, its title above it. */
export function TurnLiveView({ live }: { live: TurnLive }) {
  return (
    <figure className="armada-account__live" aria-label={live.title}>
      <figcaption className="armada-account__live-title">{live.title}</figcaption>
      <div className="armada-account__live-view">{live.view}</div>
    </figure>
  );
}

const lastDone = (lines: TurnAccount): string | undefined => lines.done[lines.done.length - 1];

/**
 * In the thread, under the message it answers: the account over the live view while the turn runs. Once it has ended it settles to one
 * quiet line, its last done line, which a press opens again.
 */
export function TurnAccountCard({ lines, live, ended }: { lines: TurnAccount; live?: TurnLive; ended: boolean }) {
  const [open, setOpen] = useState(false);
  if (ended) {
    const Fold = open ? ChevronDown : ChevronRight;
    return (
      <section className="armada-account" data-place="thread" data-settled="" aria-label="Account">
        <button type="button" className="armada-account__settled" aria-expanded={open} onClick={() => setOpen(!open)}>
          <LineMark line="done" />
          <span className="armada-account__text">{lastDone(lines)}</span>
          <Fold className="armada-account__fold" size={12} strokeWidth={2} aria-hidden />
        </button>
        {open ? (
          <div className="armada-account__body">
            <TurnAccountList lines={lines} />
            {live === undefined ? null : <TurnLiveView live={live} />}
          </div>
        ) : null}
      </section>
    );
  }
  return (
    <Card flat className="armada-account" data-place="thread" role="region" aria-label="Account">
      <TurnAccountList lines={lines} />
      {live === undefined ? null : <TurnLiveView live={live} />}
    </Card>
  );
}

/** Beside the thread, always in one place: the account over the live view. */
export function TurnAccountPanel({ lines, live }: { lines: TurnAccount; live?: TurnLive }) {
  return (
    <section className="armada-account-panel" aria-label="Now" tabIndex={0}>
      <TurnAccountList lines={lines} />
      {live === undefined ? null : <TurnLiveView live={live} />}
    </section>
  );
}

/**
 * One line on the composer's edge: what it is on now, or its last done line once the turn has ended. Pressed, it rides up as the Needs you
 * band does, to what is done, what is next and the live view, and the thread behind dims. Esc and a press on the dimmed thread fold it.
 * It shares the panel's styles, so the band and the body move the same way.
 */
export function TurnAccountBand({ lines, live, open, onOpen }: { lines: TurnAccount; live?: TurnLive; open: boolean; onOpen: (next: boolean) => void }) {
  const section = useRef<HTMLElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useScrimFold(section, open, () => onOpen(false));
  useEscapeFold(section, open, () => onOpen(false));
  const close = (event: KeyboardEvent) => {
    if (!open || event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    event.stopPropagation();
    onOpen(false);
    trigger.current?.focus();
  };
  const said = lines.now ?? lastDone(lines);
  return (
    <section ref={section} className="armada-needs-panel" aria-label="Now" data-kind="now" data-open={open} onKeyDown={close} data-keeps-escape={open ? "" : undefined}>
      <div className="armada-needs-panel__band">
        <button ref={trigger} type="button" className="armada-needs-panel__trigger" aria-label={said === undefined ? "Now" : `Now: ${said}`} aria-expanded={open} onClick={() => onOpen(!open)}>
          <LineMark line={lines.now === undefined ? "done" : "now"} />
          <span className="armada-account__band-text">{said}</span>
          <ChevronUp className="armada-needs-panel__fold" size={12} strokeWidth={2} aria-hidden />
        </button>
      </div>
      <div className="armada-needs-panel__reveal" data-open={open}>
        <div className="armada-needs-panel__shell">
          <div className="armada-needs-panel__body" data-open={open}>
            <TurnAccountList lines={{ done: lines.done, ...(lines.next === undefined ? {} : { next: lines.next }) }} />
            {live === undefined ? null : <TurnLiveView live={live} />}
          </div>
        </div>
      </div>
    </section>
  );
}
