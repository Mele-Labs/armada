import type { LucideIcon } from "lucide-react";
import { CircleCheck, CircleX, GitMerge, GitPullRequest, LoaderCircle } from "lucide-react";
import type { ReactNode } from "react";

import { Badge } from "../../primitives/Badge/Badge";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Job's pull request, small, built to the owner's list of 2 Oct 2026
 * (#1680): number and title, branch, state, Armada's Checks in one line, the
 * comment count. The whole card opens it in the browser.
 *
 * **A flat well, not a glass card** — it sits inside one. An eyebrow names it
 * a pull request before anything says which (owner, 3 Oct 2026). **Each line is drawn
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
  /**
   * The forge's own checks, and whether it was asked to merge when they pass: one mark each, its
   * tooltip naming it. **Marks, never phrases** — a live state is an animated glyph. Absent draws nothing.
   */
  forge?: PullRequestForge;
  /** What else is true of it right now — a clash with main, work not yet pushed. */
  children?: ReactNode;
  /** Opens it. The address alone never decides what opens — the host does. */
  onOpen?: () => void;
};

export type PullRequestForge = {
  checks?: "passed" | "running" | "failed";
  /** The failing checks' names, for the failed mark's tooltip. */
  failing?: readonly string[];
  /** The forge merges it when its checks pass. */
  autoMerge?: boolean;
};

const CHECKS = {
  passed: { Glyph: CircleCheck, said: "Checks passed" },
  running: { Glyph: LoaderCircle, said: "Checks running" },
  failed: { Glyph: CircleX, said: "Checks failed" },
} as const;

export function PullRequestCard({
  number,
  address,
  title,
  branch,
  state,
  checks,
  comments,
  forge,
  children,
  onOpen,
}: PullRequestCardProps) {
  const mark = forge?.checks === undefined ? undefined : CHECKS[forge.checks];
  const failing = forge?.failing ?? [];
  const checksSaid =
    mark === undefined ? "" : forge?.checks === "failed" && failing.length > 0 ? `${mark.said}: ${failing.join(", ")}` : mark.said;
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
      <span className="armada-pr-card__eyebrow">
        <GitPullRequest className="armada-pr-card__mark" size={16} strokeWidth={2} aria-hidden />
        Pull request
        {state === undefined ? null : (
          <span className="armada-pr-card__state">
            <Badge status={state.status} icon={state.icon}>
              {state.label}
            </Badge>
          </span>
        )}
      </span>
      <span className="armada-pr-card__head">
        <span className="armada-pr-card__number">{number}</span>
        {title === undefined ? null : <span className="armada-pr-card__title">{title}</span>}
        {mark === undefined && forge?.autoMerge !== true ? null : (
          <span className="armada-pr-card__forge">
            {mark === undefined ? null : (
              <Tooltip label={checksSaid}>
                <span className="armada-pr-card__check" data-checks={forge?.checks} role="img" aria-label={checksSaid}>
                  <mark.Glyph size={16} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
            {failing.length === 0 || forge?.checks !== "failed" ? null : (
              <span className="armada-pr-card__failing">{failing.join(", ")}</span>
            )}
            {forge?.autoMerge !== true ? null : (
              <Tooltip label="Auto-merge on">
                <span className="armada-pr-card__check" data-checks="auto" role="img" aria-label="Auto-merge on">
                  <GitMerge size={16} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
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
