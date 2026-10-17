import { CircleCheck, CircleX, GitMerge, GitPullRequest, LoaderCircle } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export type LedgerPullRequest = { merged: boolean; checks: "passed" | "running" | "failed"; checksSaid: string };

const PR_CHECKS = { passed: CircleCheck, running: LoaderCircle, failed: CircleX } as const;

/** A pull request's state as one bare mark: merged or open, in its hue, its tooltip naming it. */
export function PullRequestStateMark({ merged }: { merged: boolean }) {
  const said = merged ? "Merged" : "Open";
  return (
    <Tooltip label={said}>
      <span className="armada-pr-state" data-state={merged ? "merged" : "open"} role="img" aria-label={said}>
        {merged ? <GitMerge size={12} strokeWidth={2} aria-hidden /> : <GitPullRequest size={12} strokeWidth={2} aria-hidden />}
      </span>
    </Tooltip>
  );
}

/** A pull request's Checks as one bare mark, the PR card's: spinning while they run. */
export function PullRequestChecksMark({ checks, said }: { checks: LedgerPullRequest["checks"]; said: string }) {
  const Glyph = PR_CHECKS[checks];
  return (
    <Tooltip label={said}>
      <span className="armada-pr-card__check" data-checks={checks} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}
