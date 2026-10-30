import { Bot } from "lucide-react";
import type { ButtonHTMLAttributes, ReactNode } from "react";

import { Stamp } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { RepositoryChip } from "../RepositoryChip/RepositoryChip";
import { SessionMark, type SessionState } from "../SessionFrame/SessionFrame";

/**
 * The hue a Job's state wears: **ask** is owed him (waiting on him, done, a draft not started), **bad** is stuck or a Check
 * failing, **live** is a turn running, and **calm** is the rest. The Cockpit's Jobs list reads the same three it has,
 * and live is the card's own: on a card the running hue is the state, where in the list the glyph alone carries it.
 */
export type JobTone = "ask" | "bad" | "live" | "calm";

export const jobToneOf = (state: SessionState): JobTone =>
  state === "waiting" || state === "done" || state === "draft" || state === "unstarted" ? "ask" : state === "stuck" || state === "failing" ? "bad" : state === "working" ? "live" : "calm";

export type JobCardProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onClick" | "type" | "title"> & {
  title: string;
  /** The Job's state, drawn as its registry glyph. */
  state: SessionState;
  /** The state in words, the glyph's tooltip. */
  said: string;
  /** The last line the Job said. Absent, the line is left empty. */
  says?: string;
  /** The plan group it was handed from, which takes the line under the title where the last line said would be. A surface that names the Job in the group's place passes it. */
  handedFrom?: string;
  /** When it last moved, as an RFC 3339 instant or epoch milliseconds. Absent, no age is drawn. */
  at?: string | number;
  /** The repository it works in. Absent, no chip: a surface that already names the repository leaves it out. */
  repository?: string;
  selected?: boolean;
  /** Opens the Job. Other props, a roving tab stop's, go to the button. */
  onOpen: () => void;
};

/**
 * A Job as a card, where a surface names one: the Cockpit's Jobs row, framed in its state. **The state is the frame
 * and the left edge**, in the hue `jobToneOf` gives it, and its glyph with a tooltip; then the title, the last line it
 * said, its age and its repository. A bot glyph stands at the left for a Job. A press opens the Job.
 */
export function JobCard({ title, state, said, says, handedFrom, at, repository, selected = false, onOpen, ...rest }: JobCardProps) {
  return (
    <button {...rest} type="button" className="armada-job-card" data-tone={jobToneOf(state)} data-selected={selected || undefined} aria-label={`Job ${title}`} aria-current={selected || undefined} onClick={onOpen}>
      <Tooltip label="Job">
        <span className="armada-job-card__kind" role="img" aria-label="Job">
          <Bot size={14} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="armada-job-card__title">{title}</span>
      <span className="armada-job-card__mark">
        <SessionMark state={state} said={said} />
      </span>
      <span className="armada-job-card__says">{handedFrom ?? says ?? ""}</span>
      <span className="armada-job-card__at">{at === undefined ? null : <Stamp still first="age" at={at} />}</span>
      {repository === undefined ? null : (
        <span className="armada-job-card__repository">
          <RepositoryChip name={repository} />
        </span>
      )}
    </button>
  );
}

/** What a placeholder's glyph says: the Job it stands for has not been started. */
export const NOT_STARTED = "Not started yet";

export type PlaceholderJobCardProps = {
  /** The Job's title as the plan names it, or the editable stand-in a plan being drafted puts there. */
  title: ReactNode;
  /** The title in words, which names the card. */
  label: string;
  /** A surface's own control, at the title's right: the chevron that opens a plan group. */
  acts?: ReactNode;
  /** What the surface keeps for the Job until it starts, under the title and inside the frame: a plan group's issue and its tasks. */
  children?: ReactNode;
};

/**
 * A Job that is not started yet, standing where its plan group is: the Job card's frame with the outline dashed and the bot glyph muted, its tooltip **Not started
 * yet**, as `PullRequestChip` draws a pull request not opened yet. It has no state glyph, since no Job runs: `unstarted` is a Job whose agent never came up. It has
 * no age and no repository, is read and never pressed, so it is no button and no stop for Tab. Once the Job starts it is a `JobCard` in its place.
 */
export function PlaceholderJobCard({ title, label, acts, children }: PlaceholderJobCardProps) {
  return (
    <div className="armada-job-card" data-placeholder="" role="group" aria-label={`${label}, ${NOT_STARTED.toLowerCase()}`}>
      <Tooltip label={NOT_STARTED}>
        <span className="armada-job-card__kind" role="img" aria-label={`Job, ${NOT_STARTED.toLowerCase()}`}>
          <Bot size={14} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="armada-job-card__title">{title}</span>
      {acts === undefined ? null : <span className="armada-job-card__end">{acts}</span>}
      {children === undefined ? null : <div className="armada-job-card__body">{children}</div>}
    </div>
  );
}
