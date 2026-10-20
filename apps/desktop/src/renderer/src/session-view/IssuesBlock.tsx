// Progress: the issues the Session works on, each with the pull requests that close it under it, then the pull requests that close none of
// them. A pull request closing two of its issues stands under both. A press opens the issue or the pull request in the side panel.
// A plan group can name the issue it works. On the mock's `?plan=absorb` the group is that issue's place: its pull requests stand in the group
// and this block lists only the issues no group names.

import type { ReactNode } from "react";
import { CircleCheck, CircleDot } from "lucide-react";
import { PullRequestChecksMark, PullRequestStateMark, Tooltip } from "@armada/components";
import { attachmentsOf, type Session, type SessionAttachment } from "@armada/screens/src/draft/sessions";

import { checksSaid } from "../session-cards";
import { openInPanel } from "../shared-panel/store";
import "./issues-block.css";

export type Issue = Extract<SessionAttachment, { kind: "issue" }>;
export type PullRequest = Extract<SessionAttachment, { kind: "pull_request" }>;

/** What every drawing of the block is handed: the rows in order, and the two presses. */
export type IssuesDrawn = {
  issues: { issue: Issue; prs: PullRequest[] }[];
  loose: PullRequest[];
  openIssue: (issue: Issue) => void;
  openPr: (pr: PullRequest) => void;
};

export const ISSUE_STATE = { open: { Glyph: CircleDot, said: "Open" }, closed: { Glyph: CircleCheck, said: "Closed" } } as const;

/** A pull request's Checks as the PR card's mark reads them. */
export const checksOf = (pr: PullRequest): "passed" | "running" | "failed" => (pr.checks.state === "failed" ? "failed" : pr.checks.state === "pending" ? "running" : "passed");

/** The Session's issues in the order it took them, each with its pull requests, and the pull requests that close none of them. */
export function issuesOf(session: Session): { issues: { issue: Issue; prs: PullRequest[] }[]; loose: PullRequest[] } {
  const issues = attachmentsOf(session, "issue");
  const prs = attachmentsOf(session, "pull_request");
  const ours = new Set(issues.map((one) => one.number));
  return {
    issues: issues.map((issue) => ({ issue, prs: prs.filter((pr) => pr.closes?.includes(issue.number) === true) })),
    loose: prs.filter((pr) => !(pr.closes ?? []).some((number) => ours.has(number))),
  };
}

/** The issues and pull requests the block lists when the plan's groups hold the issues they name: those no group names, and the pull requests that close none of the Session's issues. */
export function issuesOutsidePlan(session: Session): { issues: { issue: Issue; prs: PullRequest[] }[]; loose: PullRequest[] } {
  const named = new Set((session.work?.plan?.groups ?? []).flatMap((group) => (group.issue === undefined ? [] : [group.issue])));
  const { issues, loose } = issuesOf(session);
  return { issues: issues.filter(({ issue }) => !named.has(issue.number)), loose };
}

/** The issue a plan group names, from the Session's attachments, with the pull requests that close it. */
export function issueOfGroup(session: Session, number: number | undefined): { issue: Issue; prs: PullRequest[] } | undefined {
  return number === undefined ? undefined : issuesOf(session).issues.find(({ issue }) => issue.number === number);
}

export const openIssuePanel = (issue: Issue, manifestId?: string) => openInPanel({ kind: "issue", number: issue.number, address: issue.address, title: issue.title, ...(manifestId === undefined ? {} : { manifestId }) });
export const openPrPanel = (pr: PullRequest) => openInPanel({ kind: "pr", number: pr.number, address: pr.address });

function PullRequestRow({ pr, openPr }: { pr: PullRequest; openPr: IssuesDrawn["openPr"] }) {
  return (
    <li>
      <button type="button" className="sv-issues__row" aria-label={`Pull request #${pr.number} ${pr.title}`} onClick={() => openPr(pr)}>
        <PullRequestStateMark merged={pr.state === "merged"} />
        <span className="sv-issues__title">
          <code>#{pr.number}</code> {pr.title}
        </span>
        <PullRequestChecksMark checks={checksOf(pr)} said={checksSaid(pr.checks)} />
      </button>
    </li>
  );
}

/** The plain list: an issue's row, its pull requests indented under it on a hairline. */
function List({ issues, loose, openIssue, openPr }: IssuesDrawn) {
  return (
    <ul className="sv-issues" aria-label="Issues">
      {issues.map(({ issue, prs }) => {
        const { Glyph, said } = ISSUE_STATE[issue.state];
        return (
          <li key={`issue${issue.number}`}>
            <button type="button" className="sv-issues__row" aria-label={`Issue #${issue.number} ${issue.title}`} onClick={() => openIssue(issue)}>
              <Tooltip label={said}>
                <span className="sv-issues__state" data-state={issue.state} role="img" aria-label={said}>
                  <Glyph size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
              <span className="sv-issues__title">
                <code>#{issue.number}</code> {issue.title}
              </span>
            </button>
            {prs.length === 0 ? null : (
              <ul className="sv-issues__prs" aria-label={`Pull requests for #${issue.number}`}>
                {prs.map((pr) => (
                  <PullRequestRow key={pr.number} pr={pr} openPr={openPr} />
                ))}
              </ul>
            )}
          </li>
        );
      })}
      {loose.map((pr) => (
        <PullRequestRow key={`pr${pr.number}`} pr={pr} openPr={openPr} />
      ))}
    </ul>
  );
}

/** What a drawing of one issue's pull requests is handed: the rows, the name of their list and the press. */
export type PrsDrawn = { prs: PullRequest[]; label: string; openPr: IssuesDrawn["openPr"] };

function ListPrs({ prs, label, openPr }: PrsDrawn) {
  return (
    <ul className="sv-issues__prs" aria-label={label}>
      {prs.map((pr) => (
        <PullRequestRow key={pr.number} pr={pr} openPr={openPr} />
      ))}
    </ul>
  );
}

// The drawing is a take while the owner chooses one on the mock (`?take=`); the one he picks replaces the others.
let drawing: (drawn: IssuesDrawn) => ReactNode = List;

/** Draws the block with another take. The mock's `?take=` calls it before the app mounts. */
export function takeIssuesDrawing(next: (drawn: IssuesDrawn) => ReactNode): void {
  drawing = next;
}

// The same take draws a plan group's pull requests, by the row component it draws its own with.
let prsDrawing: (drawn: PrsDrawn) => ReactNode = ListPrs;

/** Draws one issue's pull requests, as a plan group holds them, with another take. */
export function takePullRequestsDrawing(next: (drawn: PrsDrawn) => ReactNode): void {
  prsDrawing = next;
}

// Whether the plan's groups hold the issues they name (the mock's `?plan=absorb`) or only mark them.
let absorbing = false;

/** The plan's groups hold their issues' pull requests, and the block lists the rest. The mock's `?plan=absorb` calls it before the app mounts. */
export function absorbPlanIssues(): void {
  absorbing = true;
}

export const planAbsorbs = (): boolean => absorbing;

/** A plan group's issue, with its pull requests in the active take's rows when the plan absorbs them. */
export function GroupPullRequests({ prs, number }: { prs: PullRequest[]; number: number }) {
  const Drawing = prsDrawing;
  return <Drawing prs={prs} label={`Pull requests for #${number}`} openPr={openPrPanel} />;
}

export function IssuesBlock({ session, manifestId }: { session: Session; manifestId?: string | undefined }) {
  const { issues, loose } = absorbing ? issuesOutsidePlan(session) : issuesOf(session);
  if (issues.length === 0 && loose.length === 0) return null;
  const Drawing = drawing;
  return <Drawing issues={issues} loose={loose} openIssue={(issue) => openIssuePanel(issue, manifestId)} openPr={openPrPanel} />;
}
