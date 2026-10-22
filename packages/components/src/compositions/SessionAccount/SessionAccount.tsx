import type { ReactNode } from "react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { TASK_GLYPH, taskMarkSaid } from "../TaskMark/TaskMark";

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
