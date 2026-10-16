// The review surface's states as glyphs, each named on hover. Registry glyphs only (`packages/icons/icons`):
// `shield-*` is the Checks family, `check` and `ban` are the verdicts the pull request panel already uses, and
// `square-dashed` is a review nobody has given. **No state is a phrase.**

import type { LucideIcon } from "lucide-react";
import { Ban, Bot, Bug, Check, LoaderCircle, MessageCircleQuestion, Minus, ShieldCheck, ShieldX, SquareDashed, TriangleAlert } from "lucide-react";
import { PrMark } from "@armada/components";
import type { PullRequestReviewState } from "@armada/protocol";

import type { FindingKind, ReviewPr } from "./model";

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

export const FINDING: Record<FindingKind, Said> = {
  bug: { Glyph: Bug, said: "Bug" },
  risk: { Glyph: TriangleAlert, said: "Risk" },
  question: { Glyph: MessageCircleQuestion, said: "Question" },
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

/** Armada's own: a finding, which is never posted. */
export function ArmadaMark() {
  return <PrMark Glyph={Bot} said="Armada finding, not posted" />;
}
