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

/** Who is a reviewer on the pull request, a row each, a requested team's people under its row. Drawn only where Fleet named one. */
export function PullRequestReviewers({
  reviewers,
  onOpenLink,
}: {
  reviewers: readonly PullRequestReviewer[] | undefined;
  onOpenLink: (address: string) => void;
}) {
  if (reviewers === undefined || reviewers.length === 0) return null;
  const teams = reviewers.filter(({ kind }) => kind === "team");
  const people = reviewers.filter(({ kind }) => kind === "person");
  const placed = (slug: string) => people.filter(({ team }) => team === slug);
  const unplaced = people.filter(({ team }) => team === undefined || !teams.some(({ login }) => login === team));

  const row = ({ login, kind, verdict, name, url }: PullRequestReviewer) => {
    const { Glyph, said, tone } = VERDICT[verdict ?? "waiting"];
    // A link opens through the panel's opener, so the app itself never navigates.
    const who =
      url === undefined ? (
        <span className="armada-pr-reviewers__login">{login}</span>
      ) : (
        <button type="button" className="armada-pr-reviewers__login" data-link onClick={() => onOpenLink(url)}>
          {login}
        </button>
      );
    return (
      <>
        {name === undefined ? who : <Tooltip label={name}>{who}</Tooltip>}
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
      </>
    );
  };

  const list = (who: readonly PullRequestReviewer[]) =>
    who.length === 0 ? null : (
      <ul className="armada-pr-reviewers__list">
        {who.map((it) => (
          <li key={`${it.kind}:${it.login}`} className="armada-pr-reviewers__row">
            {row(it)}
          </li>
        ))}
      </ul>
    );

  return (
    <section className="armada-pr-plate armada-pr-reviewers" aria-label="Reviewers">
      <span className="armada-pr-plate__eyebrow">Reviewers</span>
      {teams.map((team) => (
        <div key={team.login} role="group" aria-label={team.login} className="armada-pr-reviewers__group">
          <div className="armada-pr-reviewers__row">{row(team)}</div>
          {list(placed(team.login))}
        </div>
      ))}
      {list(unplaced)}
    </section>
  );
}
