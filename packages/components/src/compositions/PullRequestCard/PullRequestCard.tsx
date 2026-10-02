import type { LucideIcon } from "lucide-react";
import { GitPullRequest } from "lucide-react";
import type { ReactNode } from "react";

import { Badge } from "../../primitives/Badge/Badge";

/**
 * A Job's pull request, small, built to the owner's list of 2 Oct 2026
 * (#1680): number and title, branch, state, Armada's Checks in one line, the
 * comment count. The whole card opens it in the browser.
 *
 * **A flat well, not a glass card** — it sits inside one. **Each line is drawn
 * only where its fact was served**: an absent prop is an absent line.
 */
export type PullRequestCardProps = {
  /** `#1750`, or the words for a pull request whose number nobody read. */
  number: string;
  /** The address it opens. Also its tooltip, so the press says where it goes. */
  address: string;
  /** Its title as the forge holds it, where the forge was read. */
  title?: string;
  /** The branch it carries. */
  branch?: string;
  /** A settled pull request's badge. Absent is open, which the mark beside the number says. */
  state?: { status: string; icon: LucideIcon; label: string };
  /** Armada's own Checks across the Job, in one line — `21/21 Checks passed`. */
  checks?: string;
  /** How many comments the pull request holds, where they were read. */
  comments?: number;
  /** What else is true of it right now — a clash with main, work not yet pushed. */
  children?: ReactNode;
  /** Opens it. The address alone never decides what opens — the host does. */
  onOpen?: () => void;
};

export function PullRequestCard({
  number,
  address,
  title,
  branch,
  state,
  checks,
  comments,
  children,
  onOpen,
}: PullRequestCardProps) {
  const facts = [
    checks,
    comments === undefined ? undefined : `${comments} ${comments === 1 ? "comment" : "comments"}`,
  ].filter((one): one is string => one !== undefined);
  return (
    <a
      className="armada-pr-card"
      href={address}
      title={address}
      aria-label={`Pull request ${number}${title === undefined ? "" : `, ${title}`}`}
      onClick={(event) => {
        event.preventDefault();
        onOpen?.();
      }}
    >
      <span className="armada-pr-card__head">
        <GitPullRequest className="armada-pr-card__mark" size={12} strokeWidth={2} aria-hidden />
        <span className="armada-pr-card__number">{number}</span>
        {title === undefined ? null : <span className="armada-pr-card__title">{title}</span>}
        {state === undefined ? null : (
          <span className="armada-pr-card__state">
            <Badge status={state.status} icon={state.icon}>
              {state.label}
            </Badge>
          </span>
        )}
      </span>
      {branch === undefined ? null : <span className="armada-pr-card__branch">{branch}</span>}
      {facts.length === 0 ? null : (
        <span className="armada-pr-card__facts">
          {facts.map((fact, at) => (
            <span key={fact}>
              {at === 0 ? null : (
                <span className="armada-pr-card__dot" aria-hidden="true">
                  ·
                </span>
              )}
              {fact}
            </span>
          ))}
        </span>
      )}
      {children}
    </a>
  );
}
