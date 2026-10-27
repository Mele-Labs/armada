// The review surface's states as glyphs, each named on hover. Registry glyphs only (`packages/icons/icons`):
// `shield-*` is the Checks family, `check` and `ban` are the verdicts the pull request panel already uses, and
// `square-dashed` is a review nobody has given. **No state is a phrase.**

import type { LucideIcon } from "lucide-react";
import { AppWindow, Ban, Bot, Briefcase, Bug, Check, CircleHelp, CircleX, ClipboardList, FlaskConical, Footprints, FolderGit2, LoaderCircle, Minus, ShieldCheck, ShieldX, SquareDashed, TriangleAlert, Unplug, UserRound, Workflow } from "lucide-react";
import { PrMark, Tooltip } from "@armada/components";
import type { PullRequestReviewState, ReviewSeverity } from "@armada/protocol";

import type { ReviewPr, ToolMark, ToolSource } from "./model";

type Said = { Glyph: LucideIcon; said: string; tone?: string };

const REVIEW: Record<PullRequestReviewState, Said> = {
  required: { Glyph: SquareDashed, said: "Review required" },
  approved: { Glyph: Check, said: "Approved", tone: "success" },
  changes_requested: { Glyph: Ban, said: "Changes requested", tone: "failed" },
  none: { Glyph: Minus, said: "No review" },
};

const CHECKS: Record<ReviewPr["checks"], Said> = {
  passed: { Glyph: ShieldCheck, said: "Checks passed", tone: "success" },
  failed: { Glyph: ShieldX, said: "Checks failed", tone: "failed" },
  running: { Glyph: LoaderCircle, said: "Checks running", tone: "running" },
};

/** A tool's own glyph. **Borrowed**: the four are the natural drawings and each is reserved elsewhere in the registry, so they wait on the owner's pick. */
export const TOOL_MARK: Record<ToolMark, Said> = {
  risks: { Glyph: TriangleAlert, said: "Risks" },
  bugs: { Glyph: Bug, said: "Possible bugs" },
  breaks: { Glyph: Unplug, said: "API breaking changes" },
  walk: { Glyph: Footprints, said: "Walk" },
  how: { Glyph: Workflow, said: "How it works" },
  impact: { Glyph: UserRound, said: "User Impact" },
  test: { Glyph: FlaskConical, said: "How to Test" },
  why: { Glyph: CircleHelp, said: "Why" },
  custom: { Glyph: ClipboardList, said: "Tool" },
};

/** Where a tool is defined, as a small glyph. Armada's is the product, his Kit is the briefcase, a repository is the repository's own. */
export const SOURCE_MARK: Record<ToolSource, Said> = {
  armada: { Glyph: AppWindow, said: "Armada's tool" },
  kit: { Glyph: Briefcase, said: "Your Kit's tool, in every repository" },
  repository: { Glyph: FolderGit2, said: "This repository's tool, shared with everyone on it" },
};

/** Where a pull request stands on review. */
export function ReviewMark({ review }: { review: PullRequestReviewState }) {
  const one = REVIEW[review];
  return <PrMark Glyph={one.Glyph} said={one.said} tone={one.tone} />;
}

/** What its checks have come to. Running turns, as the pull request card's does. */
export function ChecksMark({ checks }: { checks: ReviewPr["checks"] }) {
  const one = CHECKS[checks];
  return (
    <span className="pr-review__mark" data-checks={checks}>
      <PrMark Glyph={one.Glyph} said={one.said} tone={one.tone} />
    </span>
  );
}

/** A run that failed: Fleet's reason, where the result would be. It is never a result. */
export function RunFailed({ said }: { said: string }) {
  return (
    <p className="pr-run__failed" role="alert">
      <PrMark Glyph={CircleX} said="Failed" tone="failed" />
      {said}
    </p>
  );
}

/** How serious a finding is, as bars filled low to high. Drawn here and named on hover, since the registry holds no gauge. */
export function SeverityMark({ severity }: { severity: ReviewSeverity }) {
  return (
    <Tooltip label={`${SEVERITY[severity]} severity`}>
      <span className="pr-severity" data-severity={severity} role="img" aria-label={`${SEVERITY[severity]} severity`}>
        <i />
        <i />
        <i />
      </span>
    </Tooltip>
  );
}

const SEVERITY = { low: "Low", medium: "Medium", high: "High" } as const;

/** Armada's own: a finding, which is never posted. */
export function ArmadaMark() {
  return <PrMark Glyph={Bot} said="Armada finding, not posted" />;
}
