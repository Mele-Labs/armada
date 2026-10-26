// Landing: what the workflow ends in, so it differs by kind. A pull request draws the pull request, its Checks, its place in the merge queue and
// the merge. A spike and a document draw the doc they made. A report draws the report itself: its title and the first lines of its conclusion, and a
// press opens the whole of it in the side panel; it has no pull request and no merge. A mod draws the mods it made, each with its switch. Watching and
// Deferred stand inside, whatever the kind. **The issues a plan group works stand in the Blueprint, the pull requests in the Merge line**, and the issues the Job filed for later stand in Deferred, not on this band.
// The stage opens it (`story.ts`); the band or ⌥L does too. `work.landing` names the kind in the mock until Fleet says it.

import { Tooltip, type LedgerEntry } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { remindLater } from "../cockpit/deck/acts";
import { useArtifact } from "../session-artifact";
import { useSessionsDraft } from "../sessions-draft";
import { artifactsOf, type ArtifactRow } from "./artifacts";
import { issuesOutsidePlan } from "./IssuesBlock";
import { LaterBlock } from "./LaterBlock";
import { PullRow } from "./LedgerRow";
import { ModsMade } from "./ModsPanel";
import { SvPanel, type PanelState, type Place } from "./SvPanel";
import { WatchingBlock } from "./WatchingBlock";
import { bandKeyFor, landingOf, useFold } from "./story";
import "./story.css";

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
  const filed = issuesOutsidePlan(session).issues.map(({ issue }) => issue);
  const watching = (session.watches ?? []).length > 0;
  const mine = kind === "pull_request" ? prs.length > 0 : kind === "mod" ? true : kind === "report" ? report !== undefined : docs.length > 0;
  const empty = !mine && !watching && later.length === 0 && filed.length === 0;
  const state: PanelState | undefined =
    kind === "pull_request"
      ? prs.length === 0
        ? undefined
        : prs.every((pr) => pr.state === "merged")
          ? "done"
          : prs.some((pr) => pr.state !== "merged" && pr.checks.state === "failed")
            ? "failed"
            : "live"
      : kind === "report"
        ? report !== undefined
          ? "done"
          : undefined
        : docs.length > 0
          ? "done"
          : undefined;
  return (
    <SvPanel label="Landing" area="landing" place={place} state={state} empty={empty} fold={fold} keycap={bandKeyFor("landing")}>
      {kind === "pull_request" && prs.length > 0 ? (
        <ul className="sv-landing__prs" aria-label="Pull requests">
          {[...prs].reverse().map((pr) => (
            <PullRow key={pr.number} session={session} pr={pr} />
          ))}
        </ul>
      ) : null}
      {docs.length === 0 ? null : <Docs rows={docs} said={kind === "spike" ? "Findings" : "Document"} />}
      {report === undefined ? null : <Report session={session} row={report} />}
      {kind === "mod" ? <ModsMade id={id} /> : null}
      <WatchingBlock session={session} />
      {later.length === 0 && filed.length === 0 ? null : <LaterBlock later={later} issues={filed} manifestId={session.manifestId} onRemind={(sentence) => void remindLater(draft, id, sentence)} onDrop={(one) => draft?.dropLater?.(id, one.id)} />}
    </SvPanel>
  );
}
