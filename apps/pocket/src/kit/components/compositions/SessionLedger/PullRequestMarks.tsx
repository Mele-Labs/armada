import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { CircleCheck, LoaderCircle, CircleX } from "lucide-react";

export type LedgerPullRequest = {
  merged: boolean;
  checks: "passed" | "running" | "failed";
  checksSaid: string;
};

const PR_CHECKS = {
  passed: CircleCheck,
  running: LoaderCircle,
  failed: CircleX,
} as const;

/** A pull request's Checks as one bare mark, the PR card's: spinning while they run. */
export function PullRequestChecksMark({
  checks,
  said,
}: {
  checks: LedgerPullRequest["checks"];
  said: string;
}) {
  const Glyph = PR_CHECKS[checks];
  return (
    <Tooltip label={said}>
      <span className="armada-pr-card__check" data-checks={checks} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}
