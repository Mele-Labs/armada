// Progress: the issues the Session works on, each a single row with the pull requests that work it as rows under it, then the pull requests
// that work none of them. A pull request works an issue whose forge number it closes, or one whose row names it, as a tracker's issue's row
// does. A pull request working two of its issues stands under both. A press opens the issue or the pull request in the side
// panel. A plan group that names an issue is that issue's place: its pull requests stand in the group, and this block lists only the issues no
// group names. An issue wears the registry's `ticket` glyph and its state is its frame; a pull request's state is its edge, named by its tooltip.

import { GitMerge, GitPullRequest, Ticket } from "lucide-react";
import { PullRequestChecksMark, Tooltip } from "@armada/components";
import { attachmentsOf, issueLabel, type Session, type SessionAttachment } from "@armada/screens/src/draft/sessions";

import { usePullRequestMenu } from "../pull-request-menu";
import { checksSaid } from "../session-cards";
import { openInPanel } from "../shared-panel/store";
import "./issues-block.css";

export type Issue = Extract<SessionAttachment, { kind: "issue" }>;
export type PullRequest = Extract<SessionAttachment, { kind: "pull_request" }>;

type Rows = { issues: { issue: Issue; prs: PullRequest[] }[]; loose: PullRequest[] };

/** A pull request's Checks as the PR card's mark reads them. */
const checksOf = (pr: PullRequest): "passed" | "running" | "failed" => (pr.checks.state === "failed" ? "failed" : pr.checks.state === "pending" ? "running" : "passed");

/** Whether a pull request works an issue: it closes the issue's forge number, or the issue's row names it. */
const works = (pr: PullRequest, issue: Issue): boolean => pr.closes?.some((number) => String(number) === issue.number) === true || issue.pull_requests?.includes(pr.number) === true;

/** The Session's issues in the order it took them, each with its pull requests, and the pull requests that work none of them. */
export function issuesOf(session: Session): Rows {
  const issues = attachmentsOf(session, "issue");
  const prs = attachmentsOf(session, "pull_request");
  return {
    issues: issues.map((issue) => ({ issue, prs: prs.filter((pr) => works(pr, issue)) })),
    loose: prs.filter((pr) => !issues.some((issue) => works(pr, issue))),
  };
}

/** What the block lists, since the plan's groups hold the issues they name: those no group names, and the pull requests that work none of the Session's issues. */
export function issuesOutsidePlan(session: Session): Rows {
  const named = new Set((session.work?.plan?.groups ?? []).flatMap((group) => (group.issue === undefined ? [] : [group.issue])));
  const { issues, loose } = issuesOf(session);
  return { issues: issues.filter(({ issue }) => !named.has(issue.number)), loose };
}

/** The issue a plan group names, from the Session's attachments, with the pull requests that work it. */
export function issueOfGroup(session: Session, named: string | undefined): { issue: Issue; prs: PullRequest[] } | undefined {
  return named === undefined ? undefined : issuesOf(session).issues.find(({ issue }) => issue.number === named);
}

export const openIssuePanel = (issue: Issue, manifestId?: string) => openInPanel({ kind: "issue", number: issue.number, address: issue.address, title: issue.title, ...(manifestId === undefined ? {} : { manifestId }) });
export const openPrPanel = (pr: PullRequest) => openInPanel({ kind: "pr", number: pr.number, address: pr.address });

const ISSUE_SAID = { open: "Open", closed: "Closed" } as const;
/** A pull request's state in words, the tooltip on whatever mark carries it. */
const prSaid = (pr: PullRequest) => (pr.state === "merged" ? "Merged" : pr.state === "draft" ? "Draft" : "Open");

function IssueMark({ issue }: { issue: Issue }) {
  const said = ISSUE_SAID[issue.state];
  return (
    <Tooltip label={said}>
      <span className="sv-row__mark" data-state={issue.state} role="img" aria-label={said}>
        <Ticket size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function PrMark({ pr }: { pr: PullRequest }) {
  const Glyph = pr.state === "merged" ? GitMerge : GitPullRequest;
  return (
    <Tooltip label={prSaid(pr)}>
      <span className="sv-row__mark" data-pr={pr.state} data-checks={checksOf(pr)} role="img" aria-label={prSaid(pr)}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function PrSlot({ pr, manifestId }: { pr: PullRequest; manifestId: string | undefined }) {
  const { on, menu } = usePullRequestMenu(manifestId, pr.number, pr.title);
  return (
    <li>
      <button type="button" className="sv-row" data-kind="pr" aria-label={`Pull request #${pr.number} ${pr.title}`} onClick={() => openPrPanel(pr)} {...on}>
        <PrMark pr={pr} />
        <code className="sv-row__num">#{pr.number}</code>
        <span className="sv-row__title">{pr.title}</span>
        <PullRequestChecksMark checks={checksOf(pr)} said={checksSaid(pr.checks)} />
      </button>
      {menu}
    </li>
  );
}

/** One issue's pull requests as a plan group holds them: a row each. */
export function GroupPullRequests({ prs, issue, manifestId }: { prs: PullRequest[]; issue: string; manifestId: string | undefined }) {
  return (
    <ul className="sv-rows" aria-label={`Pull requests for ${issueLabel(issue)}`}>
      {prs.map((pr) => (
        <PrSlot key={pr.number} pr={pr} manifestId={manifestId} />
      ))}
    </ul>
  );
}

/** Each issue a single row, its state glyph, number and title, with the pull requests that work it as rows under it and the pull requests that work none after. Reference, so quieter than the plan. */
export function IssuesBlock({ session, manifestId }: { session: Session; manifestId?: string | undefined }) {
  const { issues, loose } = issuesOutsidePlan(session);
  if (issues.length === 0 && loose.length === 0) return null;
  return (
    <div className="sv-issues">
      {issues.length === 0 ? null : (
        <ul className="sv-rows" aria-label="Issues">
          {issues.map(({ issue, prs }) => (
            <li key={issue.number}>
              <button type="button" className="sv-row" data-kind="issue" aria-label={`Issue ${issueLabel(issue.number)} ${issue.title}`} onClick={() => openIssuePanel(issue, manifestId)}>
                <IssueMark issue={issue} />
                <code className="sv-row__num">{issueLabel(issue.number)}</code>
                <span className="sv-row__title">{issue.title}</span>
              </button>
              {prs.length === 0 ? null : (
                <div className="sv-rows__nested">
                  <GroupPullRequests prs={prs} issue={issue.number} manifestId={manifestId} />
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
      {loose.length === 0 ? null : (
        <ul className="sv-rows" aria-label="Pull requests">
          {loose.map((pr) => (
            <PrSlot key={pr.number} pr={pr} manifestId={manifestId} />
          ))}
        </ul>
      )}
    </div>
  );
}
