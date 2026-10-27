// Delivery: what the workflow ends in, so it differs by kind. It opens on the Job's route (`route.ts`), every step pending from the start and lit as its
// data arrives, with what the Job has made standing under the step it belongs to. A pull request draws the pull request, its Checks, its place in the merge queue and
// the merge. A spike and a document draw the doc they made. A report draws the report itself: its title and the first lines of its conclusion, and a
// press opens the whole of it in the side panel; it has no pull request and no merge. A mod draws the mods it made, each with its switch. Watching and
// Deferred stand inside, whatever the kind, and so does a page the Job wrote with the document skill, which opens in the reading layer. **The issues a plan group works stand in the Blueprint, the pull requests in the Merge line**, an issue the Job claimed stands here as a row of its own, and the issues it only filed for later stand in Deferred.
// It is open at every stage, and the larger share of the column while the stage is at it (`story.ts`); the band or ⌥D folds it by hand. `landingOf` names the kind: the workflow's `lands` from Fleet, or the mock's `work.landing`.

import { useMemo, useRef } from "react";
import { Tooltip, type LedgerEntry } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { remindLater } from "../cockpit/deck/acts";
import { useArtifact } from "../session-artifact";
import { useModsMade } from "../mods-made";
import { useSessionsDraft } from "../sessions-draft";
import { artifactsOf, type ArtifactRow } from "./artifacts";
import { DeliveryRoute } from "./DeliveryRoute";
import { documentOf } from "./document-state";
import { readPage } from "./document-page";
import { showLayer } from "./layers";
import { IssueRow, issuesOutsidePlan } from "./IssuesBlock";
import { LaterBlock } from "./LaterBlock";
import { PullRow } from "./LedgerRow";
import { stackedRows } from "./pull-stacks";
import { ModsMade } from "./ModsPanel";
import { routeOf } from "./route";
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

/**
 * The report the Job wrote: its title over the first lines of its conclusion, read from the file. A press opens the whole report as a doc
 * opens. A report that is an HTML page (the Job's document) reads its conclusion from the page's first paragraph, from the newest
 * revision, and opens in the reading layer over the Job.
 */
function Report({ session, row }: { session: Session; row: ArtifactRow }) {
  const doc = documentOf(session, row.sub);
  const held = useArtifact(useSessionsDraft(), session.id, doc === undefined ? (row.sub ?? "") : (doc.revisions[doc.revisions.length - 1] ?? ""));
  const text = held.state === "read" && held.read.ok ? new TextDecoder().decode(held.read.bytes) : "";
  const conclusion = useMemo(() => (doc === undefined ? conclusionOf(text) : text === "" ? "" : readPage(text).lede), [doc === undefined, text]);
  return (
    <button type="button" className="sv-landing__report" aria-label={row.title} disabled={row.open === undefined} onClick={doc === undefined ? row.open : () => showLayer(session.id, "document", doc.id)}>
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
  const list = useRef<HTMLUListElement>(null);
  // A chip's press lands on the parent's row, which flashes so the eye finds it among the stack.
  const jump = (number: number) => {
    const row = list.current?.querySelector<HTMLElement>(`[data-pr="${number}"]`);
    row?.querySelector<HTMLElement>(".sv-ledger__main")?.focus();
    row?.setAttribute("data-found", "");
    window.setTimeout(() => row?.removeAttribute("data-found"), 1200);
  };
  const docs = kind === "spike" || kind === "document" ? (artifactsOf(session, entries).find((group) => group.id === "doc")?.rows ?? []) : [];
  const groups = kind === "report" ? artifactsOf(session, entries) : [];
  const report = [...(groups.find((group) => group.id === "doc")?.rows ?? []), ...(groups.find((group) => group.id === "file")?.rows ?? [])].find((row) => row.sub !== undefined && row.open !== undefined);
  // The Job's own pages where the kind draws no doc of its own: a Job landing a pull request, or with no workflow, may still write one.
  const pages =
    kind === "pull_request" || kind === "mod"
      ? (artifactsOf(session, entries).find((group) => group.id === "doc")?.rows ?? []).flatMap((row) => {
          const page = row.sub === undefined ? undefined : documentOf(session, row.sub);
          return page === undefined ? [] : [{ ...row, open: () => showLayer(session.id, "document", page.id) }];
        })
      : [];
  const later = session.later ?? [];
  const outside = issuesOutsidePlan(session).issues.map(({ issue }) => issue);
  const claimed = outside.filter((issue) => issue.claimed === true);
  const filed = outside.filter((issue) => issue.claimed !== true);
  const mods = useModsMade(id).mods;
  const steps = routeOf(session, kind, { doc: kind === "report" ? report !== undefined : docs.length > 0, mods });
  const stack =
    prs.length === 0 ? undefined : (
      <ul className="sv-landing__prs" aria-label="Pull requests" ref={list}>
        {stackedRows(prs).map((row) => (
          <PullRow key={row.pr.number} session={session} pr={row.pr} onJump={jump} {...(row.lane === "none" ? {} : { lane: row.lane })} {...(row.rootBase === undefined ? {} : { rootBase: row.rootBase })} {...(row.chip === undefined ? {} : { chip: row.chip })} />
        ))}
      </ul>
    );
  const bodies = {
    "Pull request": stack,
    Findings: docs.length === 0 ? undefined : <Docs rows={docs} said="Findings" />,
    Document: kind === "report" ? (report === undefined ? undefined : <Report session={session} row={report} />) : docs.length === 0 ? undefined : <Docs rows={docs} said="Document" />,
    Mod: kind === "mod" && mods.length > 0 ? <ModsMade id={id} /> : undefined,
  };
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
    <SvPanel label="Delivery" area="landing" place={place} state={state} fold={fold} keycap={bandKeyFor("landing")}>
      <DeliveryRoute steps={steps} bodies={bodies} />
      {pages.length === 0 ? null : <Docs rows={pages} said="Document" />}
      {claimed.length === 0 ? null : (
        <ul className="sv-rows" aria-label="Issues">
          {claimed.map((issue) => (
            <li key={issue.number}>
              <IssueRow issue={issue} manifestId={session.manifestId} />
            </li>
          ))}
        </ul>
      )}
      <WatchingBlock key={session.id} session={session} />
      {later.length === 0 && filed.length === 0 ? null : <LaterBlock later={later} issues={filed} manifestId={session.manifestId} onRemind={(sentence) => void remindLater(draft, id, sentence)} onConfirm={(one) => draft?.confirmLater?.(id, one.id)} onDrop={(one) => draft?.dropLater?.(id, one.id, true)} />}
    </SvPanel>
  );
}
