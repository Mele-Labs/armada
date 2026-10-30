import { Bot } from "lucide-react";
import type { ButtonHTMLAttributes } from "react";

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

/** What a placeholder's glyph says: the Job it stands for has not been started. */
export const NOT_STARTED = "Not started yet";

export type JobCardProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onClick" | "type" | "title"> &
  (
    | {
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
        placeholder?: undefined;
      }
    | {
        /** A Job not started yet, named by the title the plan gives it. It has no state, no age and nothing to open. */
        placeholder: string;
        title?: undefined;
        state?: undefined;
        said?: undefined;
        says?: undefined;
        handedFrom?: undefined;
        at?: undefined;
        repository?: undefined;
        selected?: undefined;
        onOpen?: undefined;
      }
  );

/**
 * A Job as a card, where a surface names one: the Cockpit's Jobs row, framed in its state. **The state is the frame
 * and the left edge**, in the hue `jobToneOf` gives it, and its glyph with a tooltip; then the title, the last line it
 * said, its age and its repository. A bot glyph stands at the left for a Job. A press opens the Job.
 *
 * **A placeholder is the same card for a Job not started yet**, as `PullRequestChip` has one for a pull request not opened: the same grid, the outline dashed in the
 * quiet border with no fill, the bot glyph muted with the tooltip **Not started yet**, the title as the plan names it, and the state, the line and the age left empty.
 * It is read and never pressed, so it is no button and no stop for Tab, and it holds nothing.
 */
export function JobCard(props: JobCardProps) {
  if (props.placeholder !== undefined) {
    const { placeholder } = props;
    return (
      <div className="armada-job-card" data-placeholder="" role="group" aria-label={`${placeholder}, ${NOT_STARTED.toLowerCase()}`}>
        <Tooltip label={NOT_STARTED}>
          <span className="armada-job-card__kind" role="img" aria-label={`Job, ${NOT_STARTED.toLowerCase()}`}>
            <Bot size={14} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-job-card__title">{placeholder}</span>
        <span className="armada-job-card__mark" />
        <span className="armada-job-card__says" />
        <span className="armada-job-card__at" />
      </div>
    );
  }
  const { title, state, said, says, handedFrom, at, repository, selected = false, onOpen, ...rest } = props;
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
