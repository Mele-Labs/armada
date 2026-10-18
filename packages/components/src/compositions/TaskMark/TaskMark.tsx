import { Check, CircleDashed, CircleDot, Minus, PencilOff, X, type LucideIcon } from "lucide-react";

import { TASK_STATE } from "../../generated/vocabulary";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Task mark — the glyph a Plan region's row leads with. The mark alone
 * carries a task's state (`docs/journeys/monitor-active-work.md`, Plan); no
 * row prints a state word.
 *
 * The six are the wire's `TaskState` since 22.0, and `verbs.task_state` in
 * `enum-verbs.toml` holds the same glyphs and the words the mark is named by.
 *
 * `circle-dashed`, `pencil-off`, `minus` and `x` each carry a usage row in
 * `packages/icons/icons/` for the mark they take here. **`failed` is not
 * `dropped`**: an agent that stopped without finishing is a system failure,
 * bare `x`'s own reservation, where a drop is a person's decision. `#1535`.
 */
export type TaskMarkState = "open" | "working" | "handed_in" | "done" | "failed" | "dropped";

/** The glyph for each state — the mark's own, and the Plan task sheet's tag. */
export const TASK_GLYPH: Record<TaskMarkState, LucideIcon> = {
  open: CircleDashed,
  working: CircleDot,
  handed_in: Check,
  done: Check,
  failed: X,
  dropped: Minus,
};

/**
 * The state in the registry's own word, sentence case — the mark's accessible
 * name, and what hovering it says. **Read from `verbs.task_state` in
 * `enum-verbs.toml`, never retyped**, as `stepActivitySaid` reads a step's.
 * Every key of `TaskMarkState` has a row there, and `TaskMark.stories.tsx`
 * renders each, so a row gone missing fails a story rather than drawing a
 * mark with no name.
 */
export function taskMarkSaid(state: TaskMarkState): string {
  const verb = TASK_STATE[state]?.verb;
  if (verb === undefined || verb === null) throw new Error(`verbs.task_state has no row for ${state}`);
  return verb.charAt(0).toUpperCase() + verb.slice(1);
}

/** Task marks are 12px at strokeWidth 2, `StepActivityMark`'s own geometry. */
const MARK_ICON = 12;
const MARK_STROKE = 2;
/** The faint corner mark on a `handed_in` task: the agent's list says done and Fleet saw no edit for it. */
const UNSEEN_ICON = 8;

export type TaskMarkProps = {
  state: TaskMarkState;
};

/**
 * Never pulses — the pulse stays scoped to the running step's own mark.
 *
 * **`handed_in` reads as done**, the same check in the same hue, with a faint
 * `pencil-off` in its corner for "no edit seen". The corner mark sits out of
 * the 12px box, so a row's columns do not move.
 *
 * **A bare glyph names itself on hover** (owner's standing rule), so every
 * state takes a tooltip. `asChild`, as `StepActivityMark` attaches its `says`:
 * a wrapper would take the mark's place in its row's columns.
 */
export function TaskMark({ state }: TaskMarkProps) {
  const Icon = TASK_GLYPH[state];
  const said = taskMarkSaid(state);
  return (
    <Tooltip asChild label={said}>
      <span className="armada-task-mark" data-state={state} role="img" aria-label={said}>
        <Icon size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />
        {state === "handed_in" ? <PencilOff className="armada-task-mark__unseen" size={UNSEEN_ICON} strokeWidth={MARK_STROKE} aria-hidden /> : null}
      </span>
    </Tooltip>
  );
}
