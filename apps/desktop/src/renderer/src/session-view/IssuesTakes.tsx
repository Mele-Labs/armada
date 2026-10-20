// Three takes on the issues block, drawn in the flight deck's language, for the owner to choose between on the mock. An issue wears the
// registry's issue glyph, `ticket`, and its state is its frame; a pull request's state is its node, edge or dot, named by its tooltip.
// Objectives hangs the pull requests off a route line, as the deck hangs a handed Session off its parent. Strips frames each issue as a bay is
// framed, a band over a body, with its pull requests slotted under it. Berths draws an issue as the Bays panel draws a bay, a door down the
// left, with its pull requests as merge queue chips.

import { GitMerge, GitPullRequest, Ticket } from "lucide-react";
import type { MouseEvent } from "react";
import { PullRequestChecksMark, Tooltip } from "@armada/components";

import { checksSaid } from "../session-cards";
import { checksOf, type Issue, type IssuesDrawn, type PullRequest } from "./IssuesBlock";
import "../cockpit/deck/deck.css";
import "./issues-takes.css";

const ISSUE_SAID = { open: "Open", closed: "Closed" } as const;
const issueName = (issue: Issue) => `Issue #${issue.number} ${issue.title}`;
const prName = (pr: PullRequest) => `Pull request #${pr.number} ${pr.title}`;
/** A pull request's state in words, the tooltip on whatever mark carries it. */
const prSaid = (pr: PullRequest) => (pr.state === "merged" ? "Merged" : pr.state === "draft" ? "Draft" : "Open");

function IssueMark({ issue, size = 12 }: { issue: Issue; size?: number }) {
  const said = ISSUE_SAID[issue.state];
  return (
    <Tooltip label={said}>
      <span className="sv-take__issue-mark" data-state={issue.state} role="img" aria-label={said}>
        <Ticket size={size} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

// ---- Objectives ---------------------------------------------------------------------------------------------------

function Sortie({ pr, openPr }: { pr: PullRequest; openPr: IssuesDrawn["openPr"] }) {
  return (
    <li className="sv-obj__leg">
      <button type="button" className="sv-obj__sortie" aria-label={prName(pr)} onClick={() => openPr(pr)}>
        <Tooltip label={prSaid(pr)}>
          <span className="sv-obj__node" data-pr={pr.state} data-checks={checksOf(pr)} role="img" aria-label={prSaid(pr)} />
        </Tooltip>
        <code className="sv-obj__num">#{pr.number}</code>
        <span className="sv-obj__title">{pr.title}</span>
        <PullRequestChecksMark checks={checksOf(pr)} said={checksSaid(pr.checks)} />
      </button>
    </li>
  );
}

/** Each issue a marker in a frame its state draws, its pull requests sorties on the route under it. Those with no issue fly a dashed route. */
export function Objectives({ issues, loose, openIssue, openPr }: IssuesDrawn) {
  return (
    <div className="sv-obj">
      <ul className="sv-obj__ops" aria-label="Issues">
        {issues.map(({ issue, prs }) => (
          <li key={issue.number} className="sv-obj__op" data-state={issue.state}>
            <button type="button" className="sv-obj__target" aria-label={issueName(issue)} onClick={() => openIssue(issue)}>
              <span className="sv-obj__marker">
                <IssueMark issue={issue} />
              </span>
              <code className="sv-obj__num">#{issue.number}</code>
              <span className="sv-obj__title">{issue.title}</span>
            </button>
            {prs.length === 0 ? null : (
              <ul className="sv-obj__route" aria-label={`Pull requests for #${issue.number}`}>
                {prs.map((pr) => (
                  <Sortie key={pr.number} pr={pr} openPr={openPr} />
                ))}
              </ul>
            )}
          </li>
        ))}
      </ul>
      {loose.length === 0 ? null : (
        <ul className="sv-obj__route" data-loose="" aria-label="Pull requests">
          {loose.map((pr) => (
            <Sortie key={pr.number} pr={pr} openPr={openPr} />
          ))}
        </ul>
      )}
    </div>
  );
}

// ---- Flight strips ------------------------------------------------------------------------------------------------

function Strip({ pr, openPr }: { pr: PullRequest; openPr: IssuesDrawn["openPr"] }) {
  const Glyph = pr.state === "merged" ? GitMerge : GitPullRequest;
  return (
    <li>
      <button type="button" className="sv-strip__pr" data-pr={pr.state} data-checks={checksOf(pr)} aria-label={prName(pr)} onClick={() => openPr(pr)}>
        <Tooltip label={prSaid(pr)}>
          <span className="sv-strip__pr-mark" role="img" aria-label={prSaid(pr)}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <code className="sv-strip__pr-num">#{pr.number}</code>
        <span className="sv-strip__pr-title">{pr.title}</span>
        <PullRequestChecksMark checks={checksOf(pr)} said={checksSaid(pr.checks)} />
      </button>
    </li>
  );
}

/** Each issue a framed strip: its number on a band in the frame's hue, its title under it, its pull requests slotted in below, each with its state in its edge. */
export function Strips({ issues, loose, openIssue, openPr }: IssuesDrawn) {
  return (
    <div className="sv-strips">
      <ul className="sv-strips__board" aria-label="Issues">
        {issues.map(({ issue, prs }) => (
          <li key={issue.number} className="sv-strip" data-state={issue.state}>
            <button type="button" className="sv-strip__head" aria-label={issueName(issue)} onClick={() => openIssue(issue)}>
              <span className="sv-strip__band">
                <IssueMark issue={issue} />
                <span className="sv-strip__num">#{issue.number}</span>
              </span>
              <span className="sv-strip__title">{issue.title}</span>
            </button>
            {prs.length === 0 ? null : (
              <ul className="sv-strip__slots" aria-label={`Pull requests for #${issue.number}`}>
                {prs.map((pr) => (
                  <Strip key={pr.number} pr={pr} openPr={openPr} />
                ))}
              </ul>
            )}
          </li>
        ))}
      </ul>
      {loose.length === 0 ? null : (
        <ul className="sv-strip__slots" data-loose="" aria-label="Pull requests">
          {loose.map((pr) => (
            <Strip key={pr.number} pr={pr} openPr={openPr} />
          ))}
        </ul>
      )}
    </div>
  );
}

// ---- Berths -------------------------------------------------------------------------------------------------------

/** A pull request as the merge queue's chip: its stage as the chip's dot, or the merge glyph once it has landed, and its number. */
function Chip({ pr, openPr }: { pr: PullRequest; openPr: IssuesDrawn["openPr"] }) {
  const checks = checksOf(pr);
  const dot = checks === "failed" ? "failing" : checks === "running" ? "running" : "ready";
  const said = pr.state === "merged" ? "Merged" : checksSaid(pr.checks);
  return (
    <li>
      <Tooltip label={`${pr.title}. ${said}`}>
        <button
          type="button"
          className="deck-chip sv-berth__chip"
          aria-label={`${prName(pr)}, ${said}`}
          onClick={(event: MouseEvent) => {
            event.stopPropagation();
            openPr(pr);
          }}
        >
          {pr.state === "merged" ? <GitMerge className="sv-berth__merged" size={12} strokeWidth={2} aria-hidden /> : <span className="deck-chip__dot" data-state={dot} aria-hidden />}#{pr.number}
        </button>
      </Tooltip>
    </li>
  );
}

/** Each issue a berth: a door down the left in its state's hue with the issue glyph and its number, the title beside it, its pull requests as chips under the title. */
export function Berths({ issues, loose, openIssue, openPr }: IssuesDrawn) {
  return (
    <ul className="sv-berths" aria-label="Issues">
      {issues.map(({ issue, prs }) => (
        // A press anywhere on the berth opens the issue, as a press on a bay's tile does; the title is its button for the keys.
        <li key={issue.number} className="sv-berth" data-state={issue.state} onClick={() => openIssue(issue)}>
          <span className="sv-berth__door">
            <IssueMark issue={issue} />
            <span className="sv-berth__num">{issue.number}</span>
          </span>
          <span className="sv-berth__body">
            <button
              type="button"
              className="sv-berth__title"
              aria-label={issueName(issue)}
              onClick={(event: MouseEvent) => {
                event.stopPropagation();
                openIssue(issue);
              }}
            >
              {issue.title}
            </button>
            {prs.length === 0 ? null : (
              <ul className="sv-berth__chips" aria-label={`Pull requests for #${issue.number}`}>
                {prs.map((pr) => (
                  <Chip key={pr.number} pr={pr} openPr={openPr} />
                ))}
              </ul>
            )}
          </span>
        </li>
      ))}
      {loose.length === 0 ? null : (
        <li className="sv-berth" data-loose="">
          <span className="sv-berth__door" />
          <span className="sv-berth__body">
            <ul className="sv-berth__chips" aria-label="Pull requests">
              {loose.map((pr) => (
                <Chip key={pr.number} pr={pr} openPr={openPr} />
              ))}
            </ul>
          </span>
        </li>
      )}
    </ul>
  );
}
