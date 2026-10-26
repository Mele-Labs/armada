// Landing: what the workflow ends in, so it differs by kind. A pull request draws the pull request, its Checks, its place in the merge queue and
// the merge. A spike and a document draw the doc they made. A report draws the report itself: its title and the first lines of its conclusion, and a
// press opens the whole of it in the side panel; it has no pull request and no merge. A mod draws the mods it made, each with its switch. Watching and
// Deferred stand inside, whatever the kind. **The Job's issues are the Cockpit's Issue line and its pull requests stand in the Merge line**, not on this band.
// The stage opens it (`story.ts`); the band or ⌥L does too. `work.landing` names the kind in the mock until Fleet says it.

import { useState } from "react";
import { PullRequestActs, PullRequestChip, Tooltip, type LedgerEntry } from "@armada/components";
import { attachmentsOf, type PullRequestAct, type Session } from "@armada/screens/src/draft/sessions";

import { remindLater } from "../cockpit/deck/acts";
import { usePullRequestMenu } from "../pull-request-menu";
import { useArtifact } from "../session-artifact";
import { checksSaid } from "../session-cards";
import { useSessionsDraft } from "../sessions-draft";
import { artifactsOf, type ArtifactRow } from "./artifacts";
import { checksOf, openPrPanel, type PullRequest } from "./IssuesBlock";
import { LaterBlock } from "./LaterBlock";
import { ModsMade } from "./ModsPanel";
import { SvPanel, type PanelState, type Place } from "./SvPanel";
import { WatchingBlock } from "./WatchingBlock";
import { bandKeyFor, landingOf, useFold } from "./story";
import "./story.css";

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

/** The first paragraph of a report once its title line is past: its conclusion, as the report opens with it. */
function conclusionOf(text: string): string {
  const lines = text.split("\n").map((line) => line.trim());
  const start = lines.findIndex((line) => line !== "" && !line.startsWith("#"));
  if (start < 0) return "";
  const end = lines.findIndex((line, at) => at > start && line === "");
  return lines.slice(start, end < 0 ? undefined : end).join(" ");
}

/** The report the Job wrote: its title over the first lines of its conclusion, read from the file. A press opens the whole report as a doc opens. */
function Report({ session, row }: { session: Session; row: ArtifactRow }) {
  const held = useArtifact(useSessionsDraft(), session.id, row.sub ?? "");
  const conclusion = held.state === "read" && held.read.ok ? conclusionOf(new TextDecoder().decode(held.read.bytes)) : "";
  return (
    <button type="button" className="sv-landing__report" aria-label={row.title} disabled={row.open === undefined} onClick={row.open}>
      <span className="sv-landing__report-head">
        <Tooltip label="Report">
          <span className="sv-landing__report-mark" role="img" aria-label="Report">
            <row.Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span>{row.title}</span>
      </span>
      {conclusion === "" ? null : <span className="sv-landing__report-lines">{conclusion}</span>}
    </button>
  );
}

export function LandingPanel({ session, entries, place, id }: { session: Session; entries: readonly LedgerEntry[]; place: Place; id: string }) {
  const fold = useFold(session, "landing");
  const draft = useSessionsDraft();
  const kind = landingOf(session);
  const prs = attachmentsOf(session, "pull_request");
  const docs = kind === "spike" || kind === "document" ? (artifactsOf(session, entries).find((group) => group.id === "doc")?.rows ?? []) : [];
  const groups = kind === "report" ? artifactsOf(session, entries) : [];
  const report = [...(groups.find((group) => group.id === "doc")?.rows ?? []), ...(groups.find((group) => group.id === "file")?.rows ?? [])].find((row) => row.sub !== undefined && row.open !== undefined);
  const later = session.later ?? [];
  const watching = (session.watches ?? []).length > 0;
  const mine = kind === "pull_request" ? prs.length > 0 : kind === "mod" ? true : kind === "report" ? report !== undefined : docs.length > 0;
  const empty = !mine && !watching && later.length === 0;
  const state: PanelState | undefined = kind === "report" && report !== undefined ? "done" : kind === "pull_request" && prs.length > 0 && prs.every((pr) => pr.state === "merged") ? "done" : undefined;
  return (
    <SvPanel label="Landing" area="landing" place={place} state={state} empty={empty} fold={fold} keycap={bandKeyFor("landing")}>
      {kind === "pull_request" && prs.length > 0 ? (
        <ul className="sv-landing__prs" aria-label="Pull requests">
          {[...prs].reverse().map((pr) => (
            <Pull key={pr.number} session={session} pr={pr} />
          ))}
        </ul>
      ) : null}
      {docs.length === 0 ? null : <Docs rows={docs} said={kind === "spike" ? "Findings" : "Document"} />}
      {report === undefined ? null : <Report session={session} row={report} />}
      {kind === "mod" ? <ModsMade id={id} /> : null}
      <WatchingBlock session={session} />
      {later.length === 0 ? null : <LaterBlock later={later} onRemind={(sentence) => void remindLater(draft, id, sentence)} onDrop={(one) => draft?.dropLater?.(id, one.id)} />}
    </SvPanel>
  );
}
