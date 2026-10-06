// What a settled pull request reads as, and the badge it takes. Row, the Overview's, and the
// Job detail both use them, so neither imports the other for two words.

import { GitMerge, GitPullRequestClosed } from "lucide-react";
import type { LucideIcon } from "lucide-react";

/**
 * What a settled pull request reads as. **Written here rather than generated**,
 * unlike every status verb: `Settled` is a wire set of `crates/ipc`'s and not a
 * row in `crates/core-model/domain/`, so `enum-verbs.toml` has nothing to say
 * about it. A registry row would be the better home the day a third state
 * exists, and there is no third state — a pull request merges or it does not.
 *
 * The same two words are used on the detail, imported from here rather than
 * written twice.
 *
 * **Spelled mid-sentence, and capitalised by whoever opens a line with it.**
 * The detail continues the pull request's own fact with this — `Pull request
 * #4711, merged` — and this row opens a field with it. `leading` is what turns
 * one reading into the other, exactly as it does for the registry's verbs,
 * which are spelled lowercase for the same reason. The alternative was a second
 * roster of the same two words in mid-sentence case, which is two spellings of
 * a set that has one owner.
 */
export const LANDED: Record<string, string | undefined> = {
  merged: "merged",
  closed_unmerged: "closed without merging",
};

/**
 * The hue and glyph a settled pull request's badge takes (owner, 1 Oct 2026:
 * its state reads as a badge, the way the Job's does, and the two glyphs were
 * minted for it in `icons.toml`). **Merged is the landed hue**, the one the
 * Land board's edge already draws. **Closed without merging is neutral**:
 * nothing on the wire says why it closed, and a refusal's or a failure's hue
 * would say what nobody recorded.
 */
export const LANDED_BADGE: Record<string, { status: string; icon: LucideIcon } | undefined> = {
  merged: { status: "completed-success", icon: GitMerge },
  closed_unmerged: { status: "not-started", icon: GitPullRequestClosed },
};
