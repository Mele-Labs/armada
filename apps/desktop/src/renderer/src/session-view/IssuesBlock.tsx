// Progress: the issues the Session works on, each a framed strip with the pull requests that close it slotted under it, then the pull requests
// that close none of them. A pull request closing two of its issues stands under both. A press opens the issue or the pull request in the side
// panel. A plan group that names an issue is that issue's place: its pull requests stand in the group, and this block lists only the issues no
// group names. An issue wears the registry's `ticket` glyph and its state is its frame; a pull request's state is its edge, named by its tooltip.

import { GitMerge, GitPullRequest, Ticket } from "lucide-react";
import { PullRequestChecksMark, Tooltip } from "@armada/components";
import { attachmentsOf, type Session, type SessionAttachment } from "@armada/screens/src/draft/sessions";

import { usePullRequestMenu } from "../pull-request-menu";
import { checksSaid } from "../session-cards";
import { openInPanel } from "../shared-panel/store";
import "./issues-block.css";

export type Issue = Extract<SessionAttachment, { kind: "issue" }>;
export type PullRequest = Extract<SessionAttachment, { kind: "pull_request" }>;

type Rows = { issues: { issue: Issue; prs: PullRequest[] }[]; loose: PullRequest[] };

/** A pull request's Checks as the PR card's mark reads them. */
const checksOf = (pr: PullRequest): "passed" | "running" | "failed" => (pr.checks.state === "failed" ? "failed" : pr.checks.state === "pending" ? "running" : "passed");

/** The Session's issues in the order it took them, each with its pull requests, and the pull requests that close none of them. */
export function issuesOf(session: Session): Rows {
  const issues = attachmentsOf(session, "issue");
  const prs = attachmentsOf(session, "pull_request");
  const ours = new Set(issues.map((one) => one.number));
  return {
    issues: issues.map((issue) => ({ issue, prs: prs.filter((pr) => pr.closes?.includes(issue.number) === true) })),
    loose: prs.filter((pr) => !(pr.closes ?? []).some((number) => ours.has(number))),
  };
}

/** What the block lists, since the plan's groups hold the issues they name: those no group names, and the pull requests that close none of the Session's issues. */
export function issuesOutsidePlan(session: Session): Rows {
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

const ISSUE_SAID = { open: "Open", closed: "Closed" } as const;
/** A pull request's state in words, the tooltip on whatever mark carries it. */
const prSaid = (pr: PullRequest) => (pr.state === "merged" ? "Merged" : pr.state === "draft" ? "Draft" : "Open");

function IssueMark({ issue }: { issue: Issue }) {
  const said = ISSUE_SAID[issue.state];
  return (
    <Tooltip label={said}>
      <span className="sv-strip__issue-mark" data-state={issue.state} role="img" aria-label={said}>
        <Ticket size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function PrSlot({ pr, manifestId }: { pr: PullRequest; manifestId: string | undefined }) {
  const Glyph = pr.state === "merged" ? GitMerge : GitPullRequest;
  const { on, menu } = usePullRequestMenu(manifestId, pr.number);
  return (
    <li>
      <button type="button" className="sv-strip__pr" data-pr={pr.state} data-checks={checksOf(pr)} aria-label={`Pull request #${pr.number} ${pr.title}`} onClick={() => openPrPanel(pr)} {...on}>
        <Tooltip label={prSaid(pr)}>
          <span className="sv-strip__pr-mark" role="img" aria-label={prSaid(pr)}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <code className="sv-strip__pr-num">#{pr.number}</code>
        <span className="sv-strip__pr-title">{pr.title}</span>
        <PullRequestChecksMark checks={checksOf(pr)} said={checksSaid(pr.checks)} />
      </button>
      {menu}
    </li>
  );
}

/** One issue's pull requests as a plan group holds them: the slots alone. */
export function GroupPullRequests({ prs, number, manifestId }: { prs: PullRequest[]; number: number; manifestId: string | undefined }) {
  return (
    <ul className="sv-strip__slots" aria-label={`Pull requests for #${number}`}>
      {prs.map((pr) => (
        <PrSlot key={pr.number} pr={pr} manifestId={manifestId} />
      ))}
    </ul>
  );
}

/** Each issue a framed strip: its number on a band in the frame's hue, its title under it, its pull requests slotted in below, each with its state in its edge. */
export function IssuesBlock({ session, manifestId }: { session: Session; manifestId?: string | undefined }) {
  const { issues, loose } = issuesOutsidePlan(session);
  if (issues.length === 0 && loose.length === 0) return null;
  return (
    <div className="sv-strips">
      <ul className="sv-strips__board" aria-label="Issues">
        {issues.map(({ issue, prs }) => (
          <li key={issue.number} className="sv-strip" data-state={issue.state}>
            <button type="button" className="sv-strip__head" aria-label={`Issue #${issue.number} ${issue.title}`} onClick={() => openIssuePanel(issue, manifestId)}>
              <span className="sv-strip__band">
                <IssueMark issue={issue} />
                <span className="sv-strip__num">#{issue.number}</span>
              </span>
              <span className="sv-strip__title">{issue.title}</span>
            </button>
            {prs.length === 0 ? null : <GroupPullRequests prs={prs} number={issue.number} manifestId={manifestId} />}
          </li>
        ))}
      </ul>
      {loose.length === 0 ? null : (
        <ul className="sv-strip__slots" data-loose="" aria-label="Pull requests">
          {loose.map((pr) => (
            <PrSlot key={pr.number} pr={pr} manifestId={manifestId} />
          ))}
        </ul>
      )}
    </div>
  );
}
