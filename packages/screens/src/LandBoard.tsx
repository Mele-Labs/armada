// The Land board: what a finished Job shows, drawn from `landed.ts`. #1542.
//
// The board's arrangement: how it was answered leads, what it produced and
// what it cost read beside each other, and the tests it carries close it. The
// run, the plan and the record are the destinations under it — a board that
// redrew them would be a second copy of each, one tab away from the first.

import { Button, JobOutcome, ProducedGroups, ProducedPanel } from "@armada/components";
import type { JobOutcomePart } from "@armada/components";
import { File, Folder, GitBranch, GitCommitHorizontal, GitPullRequest } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import type { LandedPart, LandedRead } from "./landed";

/** The glyph each part takes, by the registry's own name for it. */
const MARKS: Record<NonNullable<LandedPart["mark"]>, LucideIcon> = {
  branch: GitBranch,
  commit: GitCommitHorizontal,
  "pull-request": GitPullRequest,
  // `folder` means workspace in the registry and a worktree has no row of its
  // own; `work.tsx` draws the same row the same way rather than inventing one.
  worktree: Folder,
  log: File,
};

/** Which section reads on which side. Delivered is the product; the rest is residue. */
const DELIVERED = "Delivered";

export type LandBoardProps = {
  read: LandedRead;
  /** Opens the pull request, in whatever the machine opens addresses with. */
  onOpenPullRequest: () => void;
  /** Opens the composer, which is where a follow-up is dispatched from. */
  onCompose: () => void;
  onCopied: (value: string) => void;
};

export function LandBoard({ read, onOpenPullRequest, onCompose, onCopied }: LandBoardProps) {
  const sectionOf = (section: LandedRead["sections"][number]) => ({
    name: section.name,
    ...(section.meta === undefined ? {} : { meta: section.meta }),
    parts: section.parts.map((part) => partOf(part, onOpenPullRequest)),
    ...(section.note === undefined ? {} : { note: section.note }),
  });
  const delivered = read.sections.filter((one) => one.name === DELIVERED).map(sectionOf);
  const behind = read.sections.filter((one) => one.name !== DELIVERED).map(sectionOf);

  return (
    <div className="armada-land">
      {/* How it was answered, before anything it left: a reader opening a
          finished Job came for the verdict, not for a branch name. */}
      <div className="armada-land__lead">
        <JobOutcome
          headline={{
            verb: read.verb,
            count: read.count,
            says: read.says,
            criteria: read.criteria,
            completes: read.completes,
          }}
          onCopied={onCopied}
        />
        <Button variant="secondary" ground="sunken" onClick={onCompose}>
          {read.followUp}
        </Button>
      </div>

      <div className="armada-land__columns">
        <div className="armada-land__produced">
          <JobOutcome sections={delivered} steps={read.steps} onCopied={onCopied} />
          <ProducedPanel summary={read.groupsSummary}>
            <ProducedGroups
              groups={read.groups}
              emptyNote={read.groupsAbsent}
              note={read.groupsNote}
            />
          </ProducedPanel>
        </div>

        {/* What it cost and what it left on the machine: both are residue, and
            both are read after the thing the Job was for. */}
        <div className="armada-land__cost">
          <JobOutcome cost={read.cost} sections={behind} onCopied={onCopied} />
        </div>
      </div>

      <JobOutcome runs={read.runs} onCopied={onCopied} />
    </div>
  );
}

/** One part, with its glyph and — on the pull request alone — its way out. */
function partOf(part: LandedPart, onOpenPullRequest: () => void): JobOutcomePart {
  const icon = part.mark === undefined ? undefined : MARKS[part.mark];
  return {
    name: part.name,
    ...(icon === undefined ? {} : { icon, iconLabel: part.name }),
    ...(part.value === undefined ? {} : { value: part.value }),
    ...(part.meta === undefined ? {} : { meta: part.meta }),
    ...(part.absent === undefined ? {} : { absent: part.absent }),
    ...(part.opens === undefined || part.value === undefined
      ? {}
      : {
          action: (
            <Button variant="secondary" size="sm" ground="sunken" onClick={onOpenPullRequest}>
              Open
            </Button>
          ),
        }),
  };
}
