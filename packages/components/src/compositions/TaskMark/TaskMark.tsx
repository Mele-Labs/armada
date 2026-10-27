import { Check, CircleDashed, CircleDot, Minus, PencilOff, X, type LucideIcon } from "lucide-react";

import { useNowWhile } from "../../clock";
import { TASK_STATE } from "../../generated/vocabulary";
import { ageOf } from "../../time-format";
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
  /** Fleet's proof where it set the state itself ("commit 1a2b3c4 names it"). The same mark, with the proof added to what hovering says. */
  marked?: string | undefined;
  /** A task in progress that looks left behind: the reason, and when it was last touched. The mark is dimmed, and hovering says why and how long. */
  stale?: { why: string; since: string } | undefined;
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
 *
 * **Fleet's mark and a stale one draw no new glyph** (`.claude/decisions/2026-10-27-fleet-keeps-the-task-list-honest.md`).
 * Each adds a line under the state word on hover; a stale mark is also dimmed.
 */
export function TaskMark({ state, marked, stale }: TaskMarkProps) {
  const Icon = TASK_GLYPH[state];
  const now = useNowWhile(stale !== undefined);
  const lines = [
    taskMarkSaid(state),
    ...(marked === undefined ? [] : [`Marked by Fleet: ${marked}`]),
    ...(stale === undefined ? [] : [`Stale: ${stale.why}, ${Number.isNaN(Date.parse(stale.since)) ? stale.since : ageOf(Date.parse(stale.since), now)}`]),
  ];
  const said = lines.join(". ");
  return (
    <Tooltip
      asChild
      label={
        lines.length === 1 ? (
          lines[0]
        ) : (
          <span className="armada-task-mark__says">
            {lines.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </span>
        )
      }
    >
      <span className="armada-task-mark" data-state={state} data-stale={stale === undefined ? undefined : ""} role="img" aria-label={said}>
        <Icon size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />
        {state === "handed_in" ? <PencilOff className="armada-task-mark__unseen" size={UNSEEN_ICON} strokeWidth={MARK_STROKE} aria-hidden /> : null}
      </span>
    </Tooltip>
  );
}
