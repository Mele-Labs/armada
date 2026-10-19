import type { LucideIcon } from "lucide-react";
import { Ban, Check, Clock, ScrollText, Users } from "lucide-react";
import type { PullRequestReviewer } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

type Verdict = NonNullable<PullRequestReviewer["verdict"]> | "waiting";

// The same glyphs the Conversation tab gives a verdict; a reviewer with none yet is waiting.
const VERDICT: Record<Verdict, { Glyph: LucideIcon; said: string; tone?: string }> = {
  waiting: { Glyph: Clock, said: "Waiting" },
  approved: { Glyph: Check, said: "Approved", tone: "success" },
  changes_requested: { Glyph: Ban, said: "Changes requested", tone: "failed" },
  commented: { Glyph: ScrollText, said: "Commented" },
};

/** Who is a reviewer on the pull request, a row each. Drawn only where Fleet named one. */
export function PullRequestReviewers({ reviewers }: { reviewers: readonly PullRequestReviewer[] | undefined }) {
  if (reviewers === undefined || reviewers.length === 0) return null;
  return (
    <section className="armada-pr-plate armada-pr-reviewers" aria-label="Reviewers">
      <span className="armada-pr-plate__eyebrow">Reviewers</span>
      <ul className="armada-pr-reviewers__list">
        {reviewers.map(({ login, kind, verdict }) => {
          const { Glyph, said, tone } = VERDICT[verdict ?? "waiting"];
          return (
            <li key={`${kind}:${login}`} className="armada-pr-reviewers__row">
              <span className="armada-pr-reviewers__login">{login}</span>
              {kind === "team" ? (
                <Tooltip label="Team">
                  <span className="armada-pr-reviewers__team" role="img" aria-label="Team">
                    <Users size={16} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              ) : null}
              <Tooltip label={said}>
                <span className="armada-pr-reviewers__verdict" data-tone={tone} role="img" aria-label={said}>
                  <Glyph size={16} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
