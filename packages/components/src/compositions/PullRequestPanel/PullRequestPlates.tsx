import type { LucideIcon } from "lucide-react";
import { Ban, Check, CircleCheck, CircleX, GitBranch, GitMerge, GitPullRequest, LoaderCircle, UserCheck } from "lucide-react";
import type { PullRequestMergeable, PullRequestReviewer, PullRequestReviewState } from "@armada/protocol";

import { LAND_STATE } from "../../generated/vocabulary";
import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { MergeLinePlace, type PlaceRow } from "../MergeLine/MergeLine";
import { PullRequestReviewers } from "./PullRequestReviewers";

/**
 * The plates beside a pull request's panel: its status, the merge queue it is in, and the checks
 * that failed. **Each is drawn only where Fleet served its fact**: a pull request in no queue has
 * no queue plate, and one whose checks have not failed has no failing plate.
 */
export type PullRequestChecks = { state: "pending" } | { state: "passed" } | { state: "failed"; failing: readonly string[] };

export type PullRequestPlatesProps = {
  state: "draft" | "open" | "merged";
  checks: PullRequestChecks;
  auto: boolean;
  queued: boolean;
  /** What the reviews have come to, and how many approved. Absent where the page was not read: no fact is drawn. */
  review?: PullRequestReviewState;
  approvals?: number;
  /** Who is a reviewer on it. Absent or empty draws no plate. */
  reviewers?: readonly PullRequestReviewer[];
  /** Where the forge says a merge would stand. Absent draws no fact; `unknown` draws none either. */
  mergeable?: PullRequestMergeable;
  /** The merge line the pull request is in, its own row marked current. */
  place?: readonly PlaceRow[];
  onOpenPullRequest: (address: string) => void;
  /** Tells the Session that holds it which checks failed. Absent draws no act. */
  onSendBack?: () => void;
  /** The Session has been told, for these failing checks. */
  sentBack?: boolean;
};

type Tone = "running" | "success" | "failed" | "neutral";

const WAITING = LAND_STATE.waiting?.icon ?? GitMerge;

const CHECKS = {
  pending: { Glyph: LoaderCircle, said: "Checks running", tone: "running" },
  passed: { Glyph: CircleCheck, said: "Checks passed", tone: "success" },
  failed: { Glyph: CircleX, said: "Checks failed", tone: "failed" },
} as const;

const ordinal = (place: number): string => {
  const tail = place % 100;
  const suffix = tail >= 11 && tail <= 13 ? "th" : ({ 1: "st", 2: "nd", 3: "rd" } as Record<number, string>)[place % 10] ?? "th";
  return `${place}${suffix}`;
};

/** What the status plate says, the glyph in its ring and the hue its frame takes. */
function standing(props: PullRequestPlatesProps, at: number | undefined): { what: string; Glyph: LucideIcon; tone: Tone } {
  if (props.state === "merged") return { what: "Merged", Glyph: GitMerge, tone: "success" };
  if (props.queued) return { what: at === undefined ? "In merge queue" : `${ordinal(at)} in the merge queue`, Glyph: WAITING, tone: "running" };
  if (props.state === "draft") return { what: "Draft", Glyph: GitPullRequest, tone: "neutral" };
  if (props.checks.state === "failed") return { what: "Checks failed", Glyph: CircleX, tone: "failed" };
  if (props.checks.state === "passed") return { what: "Ready to merge", Glyph: GitMerge, tone: "success" };
  return { what: "Checks running", Glyph: LoaderCircle, tone: "running" };
}

/**
 * What the reviews say, as one fact. **A request for changes outranks an approval**, since it is
 * what holds a merge; `none` and an unread page say nothing.
 */
function reviewFact({ review, approvals = 0 }: PullRequestPlatesProps) {
  if (review === "changes_requested") return { key: "review", Glyph: Ban, said: "Changes requested", tone: "failed" as const };
  if (review === "approved" || approvals >= 1) return { key: "review", Glyph: Check, said: "Approved", tone: "success" as const };
  if (review === "required") return { key: "review", Glyph: UserCheck, said: "Review required", tone: undefined };
  return undefined;
}

function conflictFact({ mergeable }: PullRequestPlatesProps) {
  if (mergeable === "conflicting") return { key: "conflicts", Glyph: GitBranch, said: "Conflicts", tone: "failed" as const };
  if (mergeable === "mergeable") return { key: "conflicts", Glyph: GitBranch, said: "No conflicts", tone: "success" as const };
  return undefined;
}

export function PullRequestPlates(props: PullRequestPlatesProps) {
  const { checks, auto, state, place, reviewers, onOpenPullRequest, onSendBack, sentBack = false } = props;
  const at = place?.find((row) => row.current === true)?.place;
  const { what, Glyph, tone } = standing(props, at);
  const mark = CHECKS[checks.state];
  const failing = checks.state === "failed" ? checks.failing : [];
  // A fact the status already says is not said twice.
  const facts = [
    ...(mark.said === what ? [] : [{ key: "checks", Glyph: mark.Glyph, said: mark.said, tone: mark.tone }]),
    ...(auto && state !== "merged" ? [{ key: "auto", Glyph: GitMerge, said: "Auto-merge on", tone: undefined }] : []),
    ...[reviewFact(props), conflictFact(props)].filter((one) => one !== undefined),
  ];
  return (
    <>
      <section className="armada-pr-plate" data-tone={tone} aria-label="Status">
        <div className="armada-pr-status">
          <span className="armada-pr-status__ring" aria-hidden>
            <Glyph size={24} strokeWidth={2} />
          </span>
          <span className="armada-pr-status__text">
            <span className="armada-pr-status__what">{what}</span>
            {failing.length === 0 || state === "merged" ? null : <span className="armada-pr-status__why">{failing.join(", ")}</span>}
          </span>
          {at === undefined ? null : (
            <Tooltip label="Place in the merge queue">
              <span className="armada-pr-status__place mono">{at}</span>
            </Tooltip>
          )}
        </div>
        {facts.length === 0 ? null : (
          <ul className="armada-pr-status__facts" aria-label="Facts">
            {facts.map(({ key, Glyph: Fact, said, tone: hue }) => (
              <li key={key} className="armada-pr-status__fact" data-tone={hue}>
                <Fact size={16} strokeWidth={2} aria-hidden />
                {said}
              </li>
            ))}
          </ul>
        )}
      </section>
      <PullRequestReviewers reviewers={reviewers} onOpenLink={onOpenPullRequest} />
      {place === undefined ? null : (
        <div className="armada-pr-plate" data-tone="running">
          <MergeLinePlace rows={place} onOpenPullRequest={onOpenPullRequest} />
        </div>
      )}
      {failing.length === 0 || state === "merged" ? null : (
        <section className="armada-pr-plate" data-tone="failed" aria-label="Failing checks">
          <span className="armada-pr-plate__eyebrow">
            <CircleX size={16} strokeWidth={2} aria-hidden />
            Failing checks
          </span>
          <ul className="armada-pr-failing">
            {failing.map((name) => (
              <li key={name} className="armada-pr-failing__row mono">
                {name}
              </li>
            ))}
          </ul>
          {onSendBack === undefined ? null : (
            <Tooltip label="Tells the Job which checks failed">
              <Button size="sm" variant="primary" disabled={sentBack} onClick={onSendBack}>
                {sentBack ? "Notified" : "Notify Job"}
              </Button>
            </Tooltip>
          )}
        </section>
      )}
    </>
  );
}
