import type { ButtonHTMLAttributes } from "react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SessionMark, type SessionState } from "../SessionFrame/SessionFrame";

export type JobChipProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onClick" | "type" | "title"> & {
  title: string;
  /** The Job's state, drawn as its registry glyph. */
  state: SessionState;
  /** The glyph's tooltip. */
  said: string;
  /** The key that opens the Job where the surface has one, shown in the tooltip. */
  shortcut?: string;
  onOpen: () => void;
};

/**
 * A Job as a chip, where a surface names one on a line of its own: its state's glyph and its title. A press opens the Job, and the tooltip says so.
 * Other props, a roving tab stop's or a mark a surface reads it by, go to the button.
 */
export function JobChip({ title, state, said, shortcut, onOpen, ...rest }: JobChipProps) {
  return (
    <Tooltip label="Open the Job" {...(shortcut === undefined ? {} : { shortcut })} asChild>
      <button {...rest} type="button" className="armada-job-chip" aria-label={`Open ${title}`} onClick={onOpen}>
        <SessionMark state={state} said={said} />
        <span className="armada-job-chip__title">{title}</span>
      </button>
    </Tooltip>
  );
}
