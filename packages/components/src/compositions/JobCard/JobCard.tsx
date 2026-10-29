import { Stamp } from "../../primitives/Stamp/Stamp";
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

export type JobCardProps = {
  title: string;
  /** The Job's state, drawn as its registry glyph. */
  state: SessionState;
  /** The state in words, the glyph's tooltip. */
  said: string;
  /** The last line the Job said. Absent, the line is left empty. */
  says?: string;
  /** When it last moved, as an RFC 3339 instant or epoch milliseconds. Absent, no age is drawn. */
  at?: string | number;
  /** The repository it works in. Absent, no chip: a surface that already names the repository leaves it out. */
  repository?: string;
  selected?: boolean;
  /** Opens the Job. */
  onOpen: () => void;
};

/**
 * A Job as a card, where a surface names one: the Cockpit's Jobs row, framed in its state. **The state is the frame
 * and the left edge**, in the hue `jobToneOf` gives it, and its glyph with a tooltip; then the title, the last line it
 * said, its age and its repository. A press opens the Job.
 */
export function JobCard({ title, state, said, says, at, repository, selected = false, onOpen }: JobCardProps) {
  return (
    <button type="button" className="armada-job-card" data-tone={jobToneOf(state)} data-selected={selected || undefined} aria-label={`Job ${title}`} aria-current={selected || undefined} onClick={onOpen}>
      <span className="armada-job-card__title">{title}</span>
      <span className="armada-job-card__mark">
        <SessionMark state={state} said={said} />
      </span>
      <span className="armada-job-card__says">{says ?? ""}</span>
      <span className="armada-job-card__at">{at === undefined ? null : <Stamp still first="age" at={at} />}</span>
      {repository === undefined ? null : (
        <span className="armada-job-card__repository">
          <RepositoryChip name={repository} />
        </span>
      )}
    </button>
  );
}
