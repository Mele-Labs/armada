import { Check, CircleDashed, CircleDot, FileCheck, Minus, X, type LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Task mark — the glyph a Plan region's row leads with. The mark alone
 * carries a task's state (`docs/journeys/monitor-active-work.md`, Plan); no
 * row prints a state word.
 *
 * The six are the wire's `TaskState` since 22.0, and `verbs.task_state` in
 * `enum-verbs.toml` holds the same glyphs. `SAID` stays written here because
 * it is sentence case, for the accessible name.
 *
 * `circle-dashed`, `file-check`, `minus` and `x` each carry a usage row in
 * `packages/icons/icons.toml` for the mark they take here. **`failed` is not
 * `dropped`**: an agent that stopped without finishing is a system failure,
 * bare `x`'s own reservation, where a drop is a person's decision. `#1535`.
 */
export type TaskMarkState = "open" | "working" | "handed_in" | "done" | "failed" | "dropped";

/** The glyph for each state — the mark's own, and the Plan task sheet's tag. */
export const TASK_GLYPH: Record<TaskMarkState, LucideIcon> = {
  open: CircleDashed,
  working: CircleDot,
  handed_in: FileCheck,
  done: Check,
  failed: X,
  dropped: Minus,
};

/** Sentence case, for the accessible name alone — no row prints these. */
const SAID: Record<TaskMarkState, string> = {
  open: "Open",
  working: "Working",
  handed_in: "Handed in",
  done: "Done",
  failed: "Failed",
  dropped: "Dropped",
};

/**
 * What hovering the mark says. **Only `handed_in` has one so far**: it arrived
 * with 22.0 and no screen draws it before slice 1b, so its tooltip changes
 * nothing anyone sees today. The other five wait on a walk.
 */
const HOVER: Partial<Record<TaskMarkState, string>> = {
  handed_in: "Handed in, waiting for its Checks",
};

/** Task marks are 12px at strokeWidth 2, `StepActivityMark`'s own geometry. */
const MARK_ICON = 12;
const MARK_STROKE = 2;

export type TaskMarkProps = {
  state: TaskMarkState;
};

/** Never pulses — the pulse stays scoped to the running step's own mark. */
export function TaskMark({ state }: TaskMarkProps) {
  const Icon = TASK_GLYPH[state];
  const mark = (
    <span className="armada-task-mark" data-state={state}>
      <Icon size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />
      <span className="armada-task-mark__name">{SAID[state]}</span>
    </span>
  );
  const hover = HOVER[state];
  return hover === undefined ? mark : <Tooltip label={hover}>{mark}</Tooltip>;
}
