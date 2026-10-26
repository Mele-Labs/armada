// The Issue line: the open Job's issues as one row of chips, directly above the Merge line and drawn as it is (same chips, same scroll across, no
// count), so a Job with five tickets takes the width of one row. A press on a chip opens the issue in the side panel as the Artifacts list does.
// A chip whose issue a pull request closes ends in that pull request's glyph and number; hovered or focused, it lights the pull request in the Merge line,
// and the Merge line's chip lights its issues the same way (`issue-partner.ts`).
// Tab reaches the chips in order, and ← and → move between them. Drawn only while a Job is open and holds an issue; a layout may hide it (`issue-line`).

import type { KeyboardEvent } from "react";
import { GitPullRequest, Ticket } from "lucide-react";
import { Tooltip } from "@armada/components";
import { attachmentsOf, issueLabel } from "@armada/screens/src/draft/sessions";
import { useLayout } from "@armada/shell";

import { works } from "../../session-view/IssuesBlock";
import { useSessions } from "../../sessions-draft";
import { openInPanel } from "../../shared-panel/store";
import { light, useLit } from "./issue-partner";
import { useDeck } from "./store";

/** ← and → move the focus along the chips; the row scrolls to keep the focused one in view. */
function stepAlong(event: KeyboardEvent<HTMLOListElement>): void {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  const chips = [...event.currentTarget.querySelectorAll<HTMLButtonElement>("button.deck-chip")];
  const at = chips.indexOf(document.activeElement as HTMLButtonElement);
  if (at < 0) return;
  event.preventDefault();
  chips[Math.max(0, Math.min(chips.length - 1, at + (event.key === "ArrowRight" ? 1 : -1)))]?.focus();
}

export function IssueLine() {
  const sessions = useSessions();
  const sel = useDeck((deck) => deck.sel);
  const shown = useLayout("dashboard.panels").shown.some((one) => one.id === "issue-line");
  const open = sel?.kind === "session" ? sessions.find((one) => `session:${one.id}` === sel.key) : undefined;
  const issues = open === undefined ? [] : attachmentsOf(open, "issue");
  const pulls = open === undefined ? [] : attachmentsOf(open, "pull_request");
  const lit = useLit();
  if (open === undefined || issues.length === 0 || !shown) return null;
  return (
    <section className="deck-panel deck-queue" aria-label="Issue line" tabIndex={-1} data-band-only="">
      <div className="deck-queue__row">
        <Tooltip label="Issue line" asChild>
          <span role="img" aria-label="Issue line">
            <Ticket size={16} aria-hidden="true" />
          </span>
        </Tooltip>
        <ol
          className="deck-chips"
          aria-label="Issues"
          onKeyDown={stepAlong}
          onWheel={(event) => {
            // A wheel turned over the line moves it along, since it scrolls only across.
            if (event.deltaY !== 0 && event.deltaX === 0) event.currentTarget.scrollLeft += event.deltaY;
          }}
        >
          {issues.map((issue) => {
            const closing = pulls.filter((pr) => works(pr, issue));
            const partner = lit?.kind === "pull" && closing.some((pr) => pr.number === lit.number);
            const lights = closing.length === 0 ? {} : { onMouseEnter: () => light({ kind: "issue", pulls: closing.map((pr) => pr.number) }), onMouseLeave: () => light(undefined), onFocus: () => light({ kind: "issue", pulls: closing.map((pr) => pr.number) }), onBlur: () => light(undefined) };
            return (
              <li key={issue.number} {...lights}>
                <Tooltip label={`${issue.title} · ${issue.state === "closed" ? "Closed" : "Open"}`} asChild>
                  <button
                    type="button"
                    className="deck-chip"
                    data-state={issue.state}
                    data-partner={partner || undefined}
                    aria-label={`Issue ${issueLabel(issue.number)} ${issue.title}${closing.map((pr) => `, closed by pull request #${pr.number}`).join("")}`}
                    onClick={() => void openInPanel({ kind: "issue", number: issue.number, address: issue.address, title: issue.title, ...(open.manifestId === undefined ? {} : { manifestId: open.manifestId }) })}
                  >
                    <Ticket size={12} aria-hidden="true" />
                    <span>{issueLabel(issue.number)}</span>
                    {closing.map((pr) => (
                      <Tooltip key={pr.number} label={`Closed by pull request #${pr.number}, ${pr.title}`}>
                        <span className="deck-chip__pull" role="img" aria-label={`Closed by pull request #${pr.number}`}>
                          <GitPullRequest size={12} aria-hidden="true" />
                          <span>#{pr.number}</span>
                        </span>
                      </Tooltip>
                    ))}
                  </button>
                </Tooltip>
              </li>
            );
          })}
        </ol>
      </div>
    </section>
  );
}
