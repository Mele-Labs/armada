// Two spellings that more than one screen reads: what a settled pull request reads as, with the
// badge it takes, and a spend written from millionths of a dollar. Kept in one place so two screens
// cannot word or round the same figure differently.

import { GitMerge, GitPullRequestClosed } from "lucide-react";
import type { LucideIcon } from "lucide-react";

/**
 * What a settled pull request reads as. **Written here rather than generated**, unlike every status
 * verb: `Settled` is a wire set of `crates/ipc`'s and not a row in `crates/core-model/domain/`, so
 * `enum-verbs.toml` has nothing to say about it. A pull request merges or it does not.
 */
const LANDED: Record<string, string | undefined> = {
  merged: "Merged",
  closed_unmerged: "Closed without merging",
};

/**
 * The hue and glyph a settled pull request's badge takes. **Merged is the landed hue**, the one the
 * merge line's edge already draws. **Closed without merging is neutral**: nothing on the wire says
 * why it closed, and a refusal's or a failure's hue would say what nobody recorded.
 */
const LANDED_BADGE: Record<string, { status: string; icon: LucideIcon } | undefined> = {
  merged: { status: "completed-success", icon: GitMerge },
  closed_unmerged: { status: "not-started", icon: GitPullRequestClosed },
};

/** A settled pull request's badge: the word and the hue beside it. */
type Settled = { status: string; icon: LucideIcon; label: string };

/** What a settled pull request reads as, as a badge. Sentence case. */
export function settledBadgeOf(landed: string | undefined): Settled | undefined {
  const label = LANDED[landed ?? ""];
  const badge = LANDED_BADGE[landed ?? ""];
  return label === undefined || badge === undefined ? undefined : { ...badge, label };
}

/**
 * A spend, from millionths of a dollar. **Four places under a cent**, so a figure that has cost a
 * tenth of a penny does not round to `~$0.00` and read as free.
 */
export function money(micros: number): string {
  const dollars = micros / 1_000_000;
  return `~$${dollars.toFixed(dollars > 0 && dollars < CENT ? 4 : 2)}`;
}

/** A cent, in dollars. Below this two decimal places round to nothing. */
const CENT = 0.01;
