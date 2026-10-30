import { Bot } from "lucide-react";
import type { ButtonHTMLAttributes } from "react";

import { Stamp } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { RepositoryChip } from "../RepositoryChip/RepositoryChip";
import { SessionMark, bucketOf, type JobBucket, type SessionState } from "../SessionFrame/SessionFrame";

/**
 * The hue a Job's state wears, by its bucket (`bucketOf`): **ask** needs him, **live** is a turn running, **wait** is
 * waiting on something that is not him, and **calm** is idle or a draft.
 */
export type JobTone = "ask" | "live" | "wait" | "calm";

const TONE: Record<JobBucket, JobTone> = { needs: "ask", working: "live", waiting: "wait", idle: "calm", draft: "calm" };

export const jobToneOf = (state: SessionState): JobTone => TONE[bucketOf(state)];

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
