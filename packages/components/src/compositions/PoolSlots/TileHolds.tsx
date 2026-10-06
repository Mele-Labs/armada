import { Anchor, Box, FilePenLine, GitBranch, GitCommitHorizontal, Link, TriangleAlert } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import type { HeldReason } from "@armada/protocol";

import { JOB_STATUS } from "../../generated/vocabulary";
import { Badge } from "../../primitives/Badge/Badge";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** The words a reason goes by on its row, in git's terms. */
function labelOf(reason: HeldReason): string {
  switch (reason.why) {
    case "not_terminal":
      return "Job status";
    case "unmerged":
      return "Unmerged commits";
    case "base_unanswered":
      return "Base branch unknown";
    case "uncommitted":
      return "Uncommitted changes";
    case "locked":
      return "Worktree locked";
    case "depended_on":
      return "Needed by unfinished Jobs";
    case "unreadable":
      return "git status failed";
  }
}

/** Every reason a worktree is held, as the phrase its tile's state names it by. */
export function holdsSaid(reasons: readonly HeldReason[]): string {
  return reasons
    .map((reason) =>
      reason.why === "not_terminal" ? "Job not finished" : labelOf(reason).replace(/^./, (one) => one.toLowerCase()),
    )
    .join(", ");
}

function glyphOf(reason: HeldReason): LucideIcon {
  switch (reason.why) {
    case "not_terminal":
      return Box;
    case "unmerged":
      return GitCommitHorizontal;
    case "base_unanswered":
      return GitBranch;
    case "uncommitted":
      return FilePenLine;
    case "locked":
      return Anchor;
    case "depended_on":
      return Link;
    case "unreadable":
      return TriangleAlert;
  }
}

/** What a reason's tooltip says: the git fact behind the word. */
function toldBy(reason: HeldReason): string {
  switch (reason.why) {
    case "not_terminal":
      return "The Job has not finished, so nothing here can be cleared";
    case "unmerged":
      return `Commits on this branch that ${reason.base} does not have`;
    case "base_unanswered":
      return "Git could not say which branch this one merges into, so it is kept";
    case "uncommitted":
      return "Changes in the worktree that no commit holds";
    case "locked":
      return "The worktree is locked with git worktree lock";
    case "depended_on":
      return "Jobs that depend on this one have not finished";
    case "unreadable":
      return "Git would not say what is in the worktree";
  }
}

const commitsOf = (n: number) => (n === 1 ? "1 commit" : `${n} commits`);

function Names({ label, items }: { label: string; items: readonly string[] }) {
  return (
    <ul className="armada-tile-holds__names" aria-label={label}>
      {items.map((one) => (
        <li key={one}>{one}</li>
      ))}
    </ul>
  );
}

/** What the reason carries beyond its word: the files, the detail. */
function Carried({ reason }: { reason: HeldReason }): ReactNode {
  switch (reason.why) {
    case "uncommitted":
      return <Names label="Uncommitted files" items={reason.files} />;
    case "depended_on":
      return <Names label="Needed by" items={reason.by} />;
    case "locked":
      return <span className="armada-tile-holds__detail">{reason.reason}</span>;
    case "base_unanswered":
    case "unreadable":
      return <span className="armada-tile-holds__detail">{reason.detail}</span>;
    case "unmerged":
    case "not_terminal":
      return null;
  }
}

/** The Job's status as the Board draws it: its badge, under a label that says whose it is. */
function JobStatus({ status, job }: { status: string; job: string | undefined }) {
  const drawn = JOB_STATUS[status];
  const verb = drawn?.verb ?? status;
  const badge =
    drawn === undefined || drawn.badgeStatus === null || drawn.icon === null || drawn.verb === null ? (
      <span className="armada-tile-holds__figure">{verb}</span>
    ) : (
      <Badge status={drawn.badgeStatus} icon={drawn.icon}>
        {drawn.verb}
      </Badge>
    );
  return (
    <>
      <span className="armada-tile-holds__word">{job === undefined ? "Job" : `Job ${job}`}</span>
      <Tooltip label={`Job status: ${verb}. It has not finished, so nothing here can be cleared`}>
        <span className="armada-tile-holds__status" role="img" aria-label={`Job status: ${verb}`}>
          {badge}
        </span>
      </Tooltip>
    </>
  );
}

/**
 * Each reason a worktree is held as one short row, a mark and a word, with
 * what it carries under it. **Every value says what it is**: the Job's status is
 * its Board badge under the Job's name, how long ago the Job last moved is said
 * in words, and commits say what they are not on. A worktree that holds nothing
 * draws nothing at all.
 */
export function TileHolds({
  reasons,
  sat,
  status,
  job,
}: {
  reasons: readonly HeldReason[];
  sat?: string | undefined;
  /** The Job's status, which the `not_terminal` row draws. */
  status?: string | undefined;
  /** The Job's handle, where the board knows it. */
  job?: string | undefined;
}) {
  if (reasons.length === 0) return null;
  return (
    <section className="armada-tile-holds" aria-label="What it holds">
      <ul className="armada-tile-holds__rows">
        {reasons.map((reason) => {
          const Glyph = glyphOf(reason);
          return (
            <li key={reason.why} className="armada-tile-holds__row">
              <div className="armada-tile-holds__line">
                <Tooltip label={toldBy(reason)}>
                  <span className="armada-tile-holds__mark" role="img" aria-label={toldBy(reason)}>
                    <Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
                {reason.why === "not_terminal" ? (
                  <JobStatus status={status ?? reason.status} job={job} />
                ) : (
                  <span className="armada-tile-holds__word">{labelOf(reason)}</span>
                )}
                {reason.why === "uncommitted" && sat !== undefined ? (
                  <Tooltip label="The files were written then or earlier">
                    <span className="armada-tile-holds__figure">{`Job last moved ${sat} ago`}</span>
                  </Tooltip>
                ) : null}
                {reason.why === "unmerged" ? (
                  <>
                    <span className="armada-tile-holds__figure">{`${commitsOf(reason.commits)} not on ${reason.base}`}</span>
                    <Tooltip label="The commit the branch points at">
                      <span className="armada-tile-holds__figure">{`tip ${reason.tip}`}</span>
                    </Tooltip>
                  </>
                ) : null}
              </div>
              <Carried reason={reason} />
            </li>
          );
        })}
      </ul>
    </section>
  );
}
