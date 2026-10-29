import type { ButtonHTMLAttributes } from "react";
import { Check, CircleDashed, CircleDot, MessageCircleQuestion } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export type TaskChipState = "open" | "held" | "asking" | "done";

/** The glyph and word the Projects card names each state by. */
const STATE: Record<TaskChipState, { Glyph: typeof CircleDot; said: string }> = {
  open: { Glyph: CircleDashed, said: "Open" },
  held: { Glyph: CircleDot, said: "In a Job" },
  asking: { Glyph: MessageCircleQuestion, said: "Done?" },
  done: { Glyph: Check, said: "Done" },
};

export type TaskChipProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onClick" | "type"> & {
  title: string;
  /** The project it is in, or "No project" where the surface says so. Absent, no project is drawn. */
  project?: string;
  state: TaskChipState;
  /** Drawn as a chip on a fill, its state on the frame and the left edge, where it stands among cards. Absent, a quiet row. */
  framed?: boolean;
  onOpen: () => void;
};

/**
 * A task of his projects, wherever the app names one: its state glyph in its hue, its title, and its project muted after it.
 * **The state is the glyph's hue** and, framed, the edge's too. A press opens it. Other props, a context menu's, go to the button.
 */
export function TaskChip({ title, project, state, framed = false, onOpen, ...rest }: TaskChipProps) {
  const { Glyph, said } = STATE[state];
  return (
    <button {...rest} type="button" className="armada-task-chip" data-state={state} data-framed={framed || undefined} aria-label={`Task ${title}`} onClick={onOpen}>
      <Tooltip label={said}>
        <span className="armada-task-chip__mark" role="img" aria-label={said}>
          <Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="armada-task-chip__words">
        <span className="armada-task-chip__title">{title}</span>
        {project === undefined ? null : <span className="armada-task-chip__project">{project}</span>}
      </span>
    </button>
  );
}
