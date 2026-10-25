// Landing: what the workflow ends in, so it differs by kind. A pull request draws the pull request, its Checks, its place in the merge queue and
// the merge. A spike and a document draw the doc they made. A mod draws the mods it made, each with its switch. Watching and Later stand inside,
// whatever the kind. **The issues and pull requests the Job works are chips on the band**, so they show with the panel folded. The stage opens it
// (`story.ts`); the band or ⌥L does too. `work.landing` names the kind in the mock until Fleet says it.

import { useState } from "react";
import { GitMerge, GitPullRequest, Ticket } from "lucide-react";
import { PullRequestActs, PullRequestChecksMark, PullRequestChip, Tooltip, type LedgerEntry } from "@armada/components";
import { attachmentsOf, issueLabel, type PullRequestAct, type Session } from "@armada/screens/src/draft/sessions";

import { remindLater } from "../cockpit/deck/acts";
import { usePullRequestMenu } from "../pull-request-menu";
import { checksSaid } from "../session-cards";
import { useSessionsDraft } from "../sessions-draft";
import { artifactsOf, type ArtifactRow } from "./artifacts";
import { checksOf, openIssuePanel, openPrPanel, type PullRequest } from "./IssuesBlock";
import { LaterBlock } from "./LaterBlock";
import { ModsMade } from "./ModsPanel";
import { SvPanel, type PanelState, type Place } from "./SvPanel";
import { WatchingBlock } from "./WatchingBlock";
import { bandKeyFor, landingOf, useFold } from "./story";
import "./story.css";

const PR_SAID = { draft: "Draft", open: "Open", merged: "Merged" } as const;

/** The issues and pull requests the Job works, a chip each: a glyph, a number, and the Checks mark on a pull request. A press opens it in the side panel. */
function Chips({ session, manifestId }: { session: Session; manifestId: string | undefined }) {
  const issues = attachmentsOf(session, "issue");
  const prs = attachmentsOf(session, "pull_request");
  if (issues.length === 0 && prs.length === 0) return null;
  return (
    <span className="sv-chips">
      {issues.map((issue) => (
        <Tooltip key={`issue:${issue.number}`} label={`${issue.title} · ${issue.state === "closed" ? "Closed" : "Open"}`} asChild>
          <button type="button" className="sv-chip" data-kind="issue" data-state={issue.state} aria-label={`Issue ${issueLabel(issue.number)}`} onClick={() => openIssuePanel(issue, manifestId)}>
            <Ticket size={12} strokeWidth={2} aria-hidden />
            <code>{issueLabel(issue.number)}</code>
          </button>
        </Tooltip>
      ))}
      {prs.map((pr) => (
        <Tooltip key={`pr:${pr.number}`} label={`${pr.title} · ${pr.queued === true ? "In merge queue" : PR_SAID[pr.state]}`} asChild>
          <button type="button" className="sv-chip" data-kind="pr" data-state={pr.state} data-checks={checksOf(pr)} aria-label={`Pull request #${pr.number}`} onClick={() => openPrPanel(pr)}>
            {pr.state === "merged" ? <GitMerge size={12} strokeWidth={2} aria-hidden /> : <GitPullRequest size={12} strokeWidth={2} aria-hidden />}
            <code>#{pr.number}</code>
            <PullRequestChecksMark checks={checksOf(pr)} said={checksSaid(pr.checks)} />
          </button>
        </Tooltip>
      ))}
    </span>
  );
}

/** One pull request: its row with the Checks mark, and under it the act that moves it on, which reads "In merge queue" once the forge has it. */
function Pull({ session, pr }: { session: Session; pr: PullRequest }) {
  const draft = useSessionsDraft();
  const [pending, setPending] = useState<PullRequestAct | undefined>(undefined);
  const { on, menu } = usePullRequestMenu(session.manifestId, pr.number, pr.title);
  const press = (act: PullRequestAct) => {
    if (draft === undefined) return;
    setPending(act);
    void draft.act(session.id, pr.number, act).finally(() => setPending(undefined));
  };
  return (
    <li className="sv-landing__pr">
      <PullRequestChip framed number={pr.number} title={pr.title} state={pr.state} checks={{ state: checksOf(pr), said: checksSaid(pr.checks) }} onOpen={() => openPrPanel(pr)} {...on} />
      {menu}
      <PullRequestActs state={pr.state} checks={pr.checks.state} auto={pr.auto} queued={pr.queued === true} {...(pending === undefined ? {} : { pending })} onAct={press} />
    </li>
  );
}

/** The docs the Job made, a spike's findings or a document: a row each, which opens the doc as the Artifacts list does. */
function Docs({ rows, said }: { rows: readonly ArtifactRow[]; said: string }) {
  return (
    <ul className="sv-landing__docs" aria-label={said}>
      {rows.map((row) => (
        <li key={row.key}>
          <button type="button" className="sv-landing__doc" aria-label={row.title} disabled={row.open === undefined} onClick={row.open}>
            <Tooltip label={said}>
              <span className="sv-landing__doc-mark" role="img" aria-label={said}>
                <row.Glyph size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            <span className="sv-landing__doc-title">{row.title}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}

export function LandingPanel({ session, entries, place, id, manifestId }: { session: Session; entries: readonly LedgerEntry[]; place: Place; id: string; manifestId?: string | undefined }) {
  const fold = useFold(session, "landing");
  const draft = useSessionsDraft();
  const kind = landingOf(session);
  const prs = attachmentsOf(session, "pull_request");
  const docs = kind === "spike" || kind === "document" ? (artifactsOf(session, entries).find((group) => group.id === "doc")?.rows ?? []) : [];
  const later = session.later ?? [];
  const watching = (session.watches ?? []).length > 0;
  const mine = kind === "pull_request" ? prs.length > 0 : kind === "mod" ? true : docs.length > 0;
  const empty = !mine && !watching && later.length === 0;
  const state: PanelState | undefined = kind === "pull_request" && prs.length > 0 && prs.every((pr) => pr.state === "merged") ? "done" : undefined;
  return (
    <SvPanel label="Landing" area="landing" place={place} state={state} acts={<Chips session={session} manifestId={manifestId} />} empty={empty} fold={fold} keycap={bandKeyFor("landing")}>
      {kind === "pull_request" && prs.length > 0 ? (
        <ul className="sv-landing__prs" aria-label="Pull requests">
          {[...prs].reverse().map((pr) => (
            <Pull key={pr.number} session={session} pr={pr} />
          ))}
        </ul>
      ) : null}
      {docs.length === 0 ? null : <Docs rows={docs} said={kind === "spike" ? "Findings" : "Document"} />}
      {kind === "mod" ? <ModsMade id={id} /> : null}
      <WatchingBlock session={session} />
      {later.length === 0 ? null : <LaterBlock later={later} onRemind={(sentence) => void remindLater(draft, id, sentence)} onDrop={(one) => draft?.dropLater?.(id, one.id)} />}
    </SvPanel>
  );
}
