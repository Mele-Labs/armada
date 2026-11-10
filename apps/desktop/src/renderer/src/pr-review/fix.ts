// A finding's fix, as a card that shows one holds it. Only a finding on the head's lines has one: a fix replaces lines
// of the branch, and the base's are not his to change.

import type { FixAsk } from "./lines";
import type { Finding, PrWork, ReviewPr } from "./model";
import { fixName, fixOf, runKey } from "./runs";
import type { ReviewStore } from "./store";

export function fixFor(
  store: Pick<ReviewStore, "spend" | "fix">,
  pr: ReviewPr,
  work: PrWork,
  finding: Finding,
  onSuggest?: (after: string) => void,
): FixAsk | undefined {
  if (finding.at.side !== "right") return undefined;
  const name = fixName(finding);
  return {
    run: work.fixes[name],
    written: fixOf(work.wire[runKey("fix", name)]),
    head: pr.head,
    onAsk: (again) => store.spend("Ask for a fix", () => store.fix(pr.key, finding, again)),
    ...(onSuggest === undefined
      ? {}
      : {
          onSuggest,
        }),
  };
}
