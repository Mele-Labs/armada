// A Session's view: its thread and composer in the middle, and little else at a glance. A Session with no `work` is drawn as it was, the thread
// alone with its ledger beside it.
//
// **What stays on screen.** The Work list, the thread and composer, the Job's story in the right column, and Flight control. The story is told
// in three zones, Done, Now and Open (`StoryColumn.tsx`), as a timeline or as three panels. The Field stands in the Cockpit's lower left, under the Jobs list. The rest is a key away: Flight control opens a layer over the stage for Subagents, Checks and Slot, and Back returns to the
// thread as it was. **Artifacts is the exception: it opens over the right column**, from the head's Artifacts icon, its key or Flight control, the
// story steps aside, and Esc returns to it. A Job that holds lifecycles also has the Workflow view in the strip: on a wide window it opens in the spine under the strip, as the others do, and on a narrower one in the side panel over the spine (`lanes-width.ts`). Pull requests stand in Done and on the Merge line. The slot and branch are drawn only when something is wrong with
// them: as a call in the thread, and by lighting the Slot key. The panels are drawn into the deck's right column (`hosts`).

import { useEffect } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import { isPressed } from "@armada/components";
import type { LedgerEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { holdsText } from "@armada/screens/src/keys";

import { useSessionsDraft } from "../sessions-draft";
import { DocumentLayer } from "./DocumentLayer";
import { FieldLayer } from "./FieldLayer";
import { ViewStrip, useJobViews } from "../cockpit/deck/JobViews";
import { StoryColumn, placeZone, type StoryZone } from "./StoryColumn";
import { BrowserBody, DiffBody, FilesBody, PlanBody, RequestBody, TerminalBody, TerminalSync } from "../job-panes/bodies";
import { JobPanes } from "../job-panes/JobPanes";
import type { Tab } from "../job-panes/tree";
import { OverTheApp } from "./OverTheApp";
import { closeLayer, dropLayer, toggleLayer, useLayer, useLayerPage } from "./layers";
import { lanesOpenFor } from "../shared-panel/store";
import { placeLanes, toggleLanes, useWideWindow } from "./lanes-width";
import type { PlanEditing } from "./PlanEdit";
import type { LiveOpen } from "./TurnAccountBlock";
import { Layer } from "./SessionLayers";
import { bandOf, isArtifactsKey, isLanesKey, isLedgerKey, isWholeKey, pressBand, pressWhole } from "./story";
import { flipLedgerView } from "./ledger-view";
import type { SessionBays } from "./slot-trouble";
import "./session-view.css";

/** The deck's own place for a Session's panels, as an element: `null` before the deck has drawn it. */
export type Hosts = { side: HTMLElement | null };

export function SessionWorkspace({ session, entries, repositories, hosts, ed, bays, onOpenSketch, onOpenSession, onOpenCheck, live, children }: { session: Session; ed: PlanEditing | undefined; entries: readonly LedgerEntry[]; repositories: readonly RepositorySummary[]; hosts?: Hosts | undefined; bays: SessionBays | undefined; onOpenSketch: (sketch: string, at?: string) => void; onOpenSession: (id: string) => void; onOpenCheck: (run: number) => void; live?: LiveOpen; children: ReactNode }) {
  const draft = useSessionsDraft();
  const { id } = session;
  const open = useLayer(id);
  const page = useLayerPage(id);
  // The Field large draws itself (`FieldLayer.tsx`), and so does the document (`DocumentLayer.tsx`); the other layers are `Layer`'s, Lanes among them on a wide window. A narrower one has Lanes in the app's side panel (`lanes-width.ts`).
  const layer = open === "field" || open === "document" ? undefined : open;
  const lanes = session.lanes;
  const large = open === "field" ? <FieldLayer session={session} repositories={repositories} /> : null;
  // A layer belongs to the Session it was opened over.
  useEffect(() => () => dropLayer(id), [id]);
  const work = session.work;
  const side = hosts?.side ?? null;
  // A Job with a story in the Cockpit has its area in panes, thread and story among them, and no column of its own for the story.
  const paned = hosts !== undefined && work !== undefined && draft !== undefined;
  const story = work !== undefined && draft !== undefined && side !== null;
  // With a story, every layer opens in the spine under the view strip; without one it covers the stage.
  const aside = story && layer !== undefined;
  const views = useJobViews(session, bays);
  // The document floats over the whole Job in a sheet of its own, drawn on the body, so it needs no place in the deck. `OverTheApp` stays
  // mounted while the Job is, so focus goes back to the opener when the document closes, and not when a dev render mounts it twice.
  const reading = <OverTheApp open={open === "document"}>{open === "document" ? <DocumentLayer key={page} session={session} /> : null}</OverTheApp>;
  const layered = layer === undefined ? null : <Layer id={layer} session={session} entries={entries} bays={bays} onOpenCheck={onOpenCheck} onOpenSession={onOpenSession} onOpenSketch={onOpenSketch} />;
  // The Lanes key toggles the view on a Job that has lanes, with or without a story. It is no flight key: the host would press the strip's button as well.
  const hasLanes = lanes !== undefined;
  // Resized across the line, open Lanes go to the place the new width puts them.
  const wide = useWideWindow();
  useEffect(() => {
    if (hasLanes) placeLanes(id);
  }, [hasLanes, id, wide]);
  useEffect(() => {
    if (!hasLanes) return;
    const listen = (event: KeyboardEvent) => {
      // In a text box ⌥ and ⇧ type a character, so the key is left to it.
      // The panel the Lanes are in is itself a dialog, which its own key closes.
      if (event.defaultPrevented || !isLanesKey(event) || holdsText(event.target) || (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null && !lanesOpenFor(id))) return;
      event.preventDefault();
      toggleLanes(id);
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, [hasLanes, id]);
  // A band opens by its key, the Blueprint's whole-plan key swaps its Now view, and the Artifacts key opens the list. A band's key closes the list first: the story is what it asked for.
  useEffect(() => {
    if (!story && !paned) return;
    const listen = (event: KeyboardEvent) => {
      if (event.defaultPrevented || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      // `message_job`: the caret goes to the Job's message box. Option changes the character M types, so the registry's reader takes the key by its place.
      if (isPressed("message_job", event)) {
        const box = document.querySelector<HTMLElement>('.sv-dock [role="textbox"][aria-label="Message"]');
        if (box === null) return;
        event.preventDefault();
        box.focus();
        window.getSelection()?.selectAllChildren(box);
        return window.getSelection()?.collapseToEnd();
      }
      if (isArtifactsKey(event)) {
        event.preventDefault();
        return toggleLayer(id, "artifacts");
      }
      if (isLedgerKey(event)) {
        event.preventDefault();
        if (layer === "artifacts") closeLayer();
        return flipLedgerView();
      }
      if (isWholeKey(event) && session.work?.plan !== undefined) {
        event.preventDefault();
        if (layer === "artifacts") closeLayer();
        pressWhole(session);
        return;
      }
      const band = bandOf(event);
      if (band === undefined) return;
      event.preventDefault();
      if (layer === "artifacts") closeLayer();
      if (band.area === "workflow") return document.querySelector<HTMLElement>(".sv-side [data-workflow-pick]")?.focus();
      pressBand(session, band.area);
      window.setTimeout(() => {
        const header = document.querySelector<HTMLElement>(`[data-area="${band.area}"] > .sv-panel__band`);
        if (header === null) return;
        // The zones are one scrolling story: the zone comes to the top of it.
        const zone = header.parentElement;
        header.focus({ preventScroll: true });
        if (zone !== null) placeZone(zone, "smooth");
      }, 0);
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, [story, paned, id, layer, session]);
  if (paned) {
    const zone = (only?: StoryZone) => (
      <div className="sv-story-body">
        <StoryColumn session={session} entries={entries} work={work} id={id} manifestId={session.manifestId} acts={draft.work} ed={ed} onOpenSketch={onOpenSketch} onOpenSession={onOpenSession} live={live} only={only} />
      </div>
    );
    const bodyOf = (tab: Tab) => {
      switch (tab.kind) {
        case "thread":
          return children;
        case "done":
          return zone("done");
        case "now":
          return zone("now");
        case "open":
          return zone("next");
        case "spine":
          return zone();
        case "browser":
          return <BrowserBody jobId={id} />;
        case "terminal":
          return <TerminalBody jobId={id} />;
        case "diff":
          return <DiffBody />;
        case "plan":
          return <PlanBody session={session} />;
        case "files":
          return <FilesBody entries={entries} />;
        case "request":
          return <RequestBody session={session} />;
        case "system":
          return <Layer id="system" session={session} entries={entries} bays={bays} onOpenCheck={onOpenCheck} onOpenSession={onOpenSession} onOpenSketch={onOpenSketch} />;
        case "bays":
          return <Layer id="slot" session={session} entries={entries} bays={bays} onOpenCheck={onOpenCheck} onOpenSession={onOpenSession} onOpenSketch={onOpenSketch} />;
      }
    };
    return (
      <>
        <JobPanes jobId={id} body={bodyOf} bar={views === undefined ? null : <ViewStrip views={views} />} over={layered} />
        <TerminalSync jobId={id} />
        {large}
        {reading}
      </>
    );
  }
  // A Job with no story still has its views: the strip stands in the column, and a view opens under it.
  if (!story) {
    if (views === undefined || side === null) return <>{children}{layered}{large}</>;
    return (
      <>
        {children}
        {large}
        {createPortal(
          <>
            <ViewStrip views={views} />
            {layered === null ? null : <div className="sv-story-body">{layered}</div>}
          </>,
          side,
        )}
      </>
    );
  }
  // The story is three zones in the order a layout gives; each leaves itself out where a layout hides it (`SvPanel`). The workflow's picker stands above them.
  const right = <StoryColumn session={session} entries={entries} work={work} id={id} manifestId={session.manifestId} acts={draft.work} ed={ed} onOpenSketch={onOpenSketch} onOpenSession={onOpenSession} live={live} />;
  return (
    <>
      {children}
      {aside ? null : layered}
      {large}
      {reading}
      {createPortal(
        <>
          {views === undefined ? null : <ViewStrip views={views} />}
          <div className="sv-story-body">
            {right}
            {aside ? layered : null}
          </div>
        </>,
        side,
      )}
    </>
  );
}
