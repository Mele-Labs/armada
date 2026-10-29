import type { ButtonHTMLAttributes } from "react";
import { GitMerge, GitPullRequest, GitPullRequestClosed } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PullRequestChecksMark, type LedgerPullRequest } from "../SessionLedger/PullRequestMarks";

export type PullRequestChipState = "open" | "draft" | "merged" | "closed";

const STATE: Record<PullRequestChipState, { Glyph: typeof GitPullRequest; said: string }> = {
  open: { Glyph: GitPullRequest, said: "Open" },
  draft: { Glyph: GitPullRequest, said: "Draft" },
  merged: { Glyph: GitMerge, said: "Merged" },
  closed: { Glyph: GitPullRequestClosed, said: "Closed" },
};

/** What a placeholder's glyph says: the pull request it stands for has no number yet. */
export const NOT_OPENED = "Not opened yet";

export type PullRequestChipProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onClick" | "type"> & {
  /**
   * Drawn as a chip on a fill, its state on the frame and the left edge, where it stands among cards. Absent, it is the
   * quiet row a Job's Progress lists its pull requests in.
   */
  framed?: boolean;
} & (
    | {
        number: number;
        title: string;
        state: PullRequestChipState;
        /** Its Checks, where the surface knows them, with their words for the tooltip. Absent, no Checks mark. */
        checks?: { state: LedgerPullRequest["checks"]; said: string };
        placeholder?: undefined;
        onOpen: () => void;
      }
    | {
        /** A pull request not opened yet, named by this label (`PR 1`) where the title goes. It has no number and nothing to open. */
        placeholder: string;
        number?: undefined;
        title?: undefined;
        state?: undefined;
        checks?: undefined;
        onOpen?: undefined;
      }
  );

/**
 * A pull request as a Job's Progress lists it: its state glyph in its hue, its number in mono, its title, and its Checks
 * mark. **The state is the glyph's hue** (merged the advanced hue, a Check running the running hue, one failing red)
 * and, framed, the edge's too. A press opens it. Other props, a context menu's, go to the button.
 *
 * **A placeholder is the same shape with a dashed outline**, the glyph muted, `#····` in the number's place and its label
 * in the title's. It is read and never pressed, so it is no button and no stop for Tab.
 */
export function PullRequestChip(props: PullRequestChipProps) {
  if (props.placeholder !== undefined) {
    const { placeholder, framed = false } = props;
    return (
      <span className="armada-pull-chip" data-placeholder="" data-framed={framed || undefined} role="group" aria-label={`${placeholder}, ${NOT_OPENED.toLowerCase()}`}>
        <Tooltip label={NOT_OPENED}>
          <span className="armada-pull-chip__mark" role="img" aria-label={NOT_OPENED}>
            <GitPullRequest size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <code className="armada-pull-chip__number">#····</code>
        <span className="armada-pull-chip__title">{placeholder}</span>
        <span />
      </span>
    );
  }
  const { number, title, state, checks, framed = false, onOpen, ...rest } = props;
  const { Glyph, said } = STATE[state];
  return (
    <button {...rest} type="button" className="armada-pull-chip" data-state={state} data-checks={checks?.state} data-framed={framed || undefined} aria-label={`Pull request #${number} ${title}`} onClick={onOpen}>
      <Tooltip label={said}>
        <span className="armada-pull-chip__mark" role="img" aria-label={said}>
          <Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <code className="armada-pull-chip__number">#{number}</code>
      <span className="armada-pull-chip__title">{title}</span>
      {checks === undefined ? <span /> : <PullRequestChecksMark checks={checks.state} said={checks.said} />}
    </button>
  );
}
