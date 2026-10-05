import { Anchor, FilePenLine, GitBranch, GitCommitHorizontal, Link, TriangleAlert } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import type { HeldReason } from "@armada/protocol";

import { JOB_STATUS } from "../../generated/vocabulary";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** The word a reason goes by: on its row, and in its tile's state. */
function labelOf(reason: HeldReason): string {
  switch (reason.why) {
    case "not_terminal":
      return JOB_STATUS[reason.status]?.verb ?? reason.status;
    case "unmerged":
      return "Unmerged";
    case "base_unanswered":
      return "Base unanswered";
    case "uncommitted":
      return "Uncommitted";
    case "locked":
      return "Locked";
    case "depended_on":
      return "Depended on";
    case "unreadable":
      return "Unreadable";
  }
}

/** Every reason a worktree is held, as the words its tile's state names it by. */
export function holdsSaid(reasons: readonly HeldReason[]): string {
  return reasons.map((reason) => labelOf(reason).toLowerCase()).join(", ");
}

function glyphOf(reason: HeldReason): LucideIcon {
  switch (reason.why) {
    case "not_terminal":
      return JOB_STATUS[reason.status]?.icon ?? Anchor;
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

/** What a reason's tooltip names: the test the worktree did not pass. */
function toldBy(reason: HeldReason): string {
  switch (reason.why) {
    case "not_terminal":
      return "The Job has not ended";
    case "unmerged":
      return `Commits ${reason.base} cannot reach`;
    case "base_unanswered":
      return "Nothing could name the base";
    case "uncommitted":
      return "Written and committed nowhere";
    case "locked":
      return "Locked";
    case "depended_on":
      return "Another Job has not finished";
    case "unreadable":
      return "Version control would not say";
  }
}

function Names({ label, items }: { label: string; items: readonly string[] }) {
  return (
    <ul className="armada-holds__names" aria-label={label}>
      {items.map((one) => (
        <li key={one}>{one}</li>
      ))}
    </ul>
  );
}

/** What the reason carries beyond its word: the files, the tip, the detail. */
function Carried({ reason }: { reason: HeldReason }): ReactNode {
  switch (reason.why) {
    case "uncommitted":
      return <Names label="Uncommitted files" items={reason.files} />;
    case "depended_on":
      return <Names label="Depended on by" items={reason.by} />;
    case "locked":
      return <span className="armada-holds__detail">{reason.reason}</span>;
    case "base_unanswered":
    case "unreadable":
      return <span className="armada-holds__detail">{reason.detail}</span>;
    case "unmerged":
    case "not_terminal":
      return null;
  }
}

/**
 * Each reason a worktree is held as one short row, a mark and a word, with what
 * it carries under it. **How long the files have sat is on the Uncommitted
 * row**, the one reason where a Clear ends something. A worktree that holds
 * nothing draws nothing at all.
 */
export function TileHolds({ reasons, sat }: { reasons: readonly HeldReason[]; sat?: string | undefined }) {
  if (reasons.length === 0) return null;
  return (
    <section className="armada-holds" aria-label="What it holds">
      <ul className="armada-holds__rows">
        {reasons.map((reason) => {
          const Glyph = glyphOf(reason);
          return (
            <li key={reason.why} className="armada-holds__row">
              <div className="armada-holds__line">
                <Tooltip label={toldBy(reason)}>
                  <span className="armada-holds__mark" role="img" aria-label={toldBy(reason)}>
                    <Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
                <span className="armada-holds__word">{labelOf(reason)}</span>
                {reason.why === "uncommitted" && sat !== undefined ? (
                  <Tooltip label={`Last moved ${sat} ago`}>
                    <span className="armada-holds__figure" aria-label={`Last moved ${sat} ago`}>
                      {sat}
                    </span>
                  </Tooltip>
                ) : null}
                {reason.why === "unmerged" ? (
                  <>
                    <Tooltip label={`${reason.commits === 1 ? "1 commit" : `${reason.commits} commits`} not on ${reason.base}`}>
                      <span className="armada-holds__figure" aria-label={`${reason.commits === 1 ? "1 commit" : `${reason.commits} commits`} not on ${reason.base}`}>
                        {reason.commits === 1 ? "1 commit" : `${reason.commits} commits`}
                      </span>
                    </Tooltip>
                    <Tooltip label="Tip">
                      <span className="armada-holds__figure" aria-label={`Tip ${reason.tip}`}>
                        {reason.tip}
                      </span>
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
