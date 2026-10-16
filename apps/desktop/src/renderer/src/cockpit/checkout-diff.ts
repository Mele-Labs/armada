// An uncommitted file of a held checkout, opened from "Your checkout": Fleet's diff for it and the diff that draws as. Apart from
// `YourCheckout.tsx`, which holds the open lines.

import type { UnifiedDiffProps } from "@armada/components";
import { drawnOf } from "@armada/screens/src/drawn-patch";
import type { CheckoutDiffRead } from "@armada/screens/src/draft/sessions";

/** What a cut diff says. A diff is bounded in the window; `git diff` in the repository has the rest. */
const cutSentence = (drawnLines: number, total: number): string => `The first ${drawnLines.toLocaleString()} of ${total.toLocaleString()} lines.`;

const NO_CHANGE = "No lines changed in this file.";

/**
 * The diff for one file. **Empty with no note while the read is in flight**; a refusal says Fleet's own sentence, and a file git
 * finds no difference in says there is none.
 */
export function checkoutDiffOf(read: CheckoutDiffRead | undefined): UnifiedDiffProps {
  if (read === undefined) return { files: [] };
  if (!read.ok) return { files: [], emptyNote: read.said };
  const drawn = drawnOf(read.diff, cutSentence);
  if (drawn.files.length === 0) return { files: [], emptyNote: NO_CHANGE };
  return { files: drawn.files, ...(drawn.cut === undefined ? {} : { cut: drawn.cut }) };
}
