// A Session's view: its thread and composer in the middle, and little else at a glance. A Session with no `work` is drawn as it was, the thread
// alone with its ledger beside it.
//
// **What stays on screen.** The Work list, the thread and composer, the Job's story in the right column, and Flight control. The story is told
// top to bottom: the Workflow, the Blueprint (the plan), the Field and the Landing, the one the stage is at open and the rest folded to their
// bands (`story.ts`). The rest is a key away: Flight control opens a layer over the stage for Subagents, Checks and Slot, and Back returns to the
// thread as it was. **Artifacts is the exception: it opens over the right column**, from the composer's Artifacts key or from Flight control, the
// story steps aside, and Esc returns to it. Pull requests stand in the Landing and on the Merge line. The slot and branch are drawn only when something is wrong with
// them: as a call in the thread, and by lighting the Slot key. The panels are drawn into the deck's right column (`hosts`).

import { Fragment, useEffect } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import type { LedgerEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { useLayout } from "@armada/shell";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { DocumentLayer } from "./DocumentLayer";
import { FieldLayer } from "./FieldLayer";
import { FieldPanel } from "./FieldPanel";
import { BlueprintPanel } from "./BlueprintPanel";
import { LandingPanel } from "./LandingPanel";
import { OverTheApp } from "./OverTheApp";
import { closeLayer, dropLayer, toggleLayer, useLayer, useLayerPage } from "./layers";
import type { PlanEditing } from "./PlanEdit";
import { WorkflowPanel } from "./WorkflowPanel";
import type { LiveOpen } from "./TurnAccountBlock";
import { Layer } from "./SessionLayers";
import { bandOf, isArtifactsKey, isWholeKey, pressBand, pressWhole, resetFolds, stageOf } from "./story";
import type { SessionBays } from "./slot-trouble";
import "./session-view.css";

/** The deck's own place for a Session's panels, as an element: `null` before the deck has drawn it. */
export type Hosts = { side: HTMLElement | null };

export function SessionWorkspace({ session, entries, repositories, hosts, ed, bays, onOpenSketch, onOpenSession, onOpenCheck, live, children }: { session: Session; ed: PlanEditing | undefined; entries: readonly LedgerEntry[]; repositories: readonly RepositorySummary[]; hosts?: Hosts | undefined; bays: SessionBays | undefined; onOpenSketch: (sketch: string, at?: string) => void; onOpenSession: (id: string) => void; onOpenCheck: (run: number) => void; live?: LiveOpen; children: ReactNode }) {
  const draft = useSessionsDraft();
  const order = useLayout("job.panels").all;
  const { id } = session;
  const open = useLayer(id);
  const page = useLayerPage(id);
  // The Field large draws itself (`FieldLayer.tsx`) and so does the document (`DocumentLayer.tsx`); the other layers are `Layer`'s.
  const layer = open === "field" || open === "document" ? undefined : open;
  const large = open === "field" ? <FieldLayer session={session} repositories={repositories} /> : null;
  // A layer belongs to the Session it was opened over.
  useEffect(() => () => dropLayer(id), [id]);
  const work = session.work;
  const stage = stageOf(session);
  // What the person opened or folded by hand holds until the story moves on.
  useEffect(() => resetFolds(id), [id, stage]);
  const side = hosts?.side ?? null;
  const story = work !== undefined && draft !== undefined && side !== null;
  // Artifacts covers the right column, where the story is; every other layer covers the stage.
  const aside = story && layer === "artifacts";
  // The document floats over the whole Job in a sheet of its own, drawn on the body, so it needs no place in the deck. `OverTheApp` stays
  // mounted while the Job is, so focus goes back to the opener when the document closes, and not when a dev render mounts it twice.
  const reading = <OverTheApp open={open === "document"}>{open === "document" ? <DocumentLayer key={page} session={session} /> : null}</OverTheApp>;
  const layered = layer === undefined ? null : <Layer id={layer} session={session} entries={entries} bays={bays} onOpenCheck={onOpenCheck} onOpenSession={onOpenSession} onOpenSketch={onOpenSketch} />;
  // A band opens by its key, the Blueprint's whole-plan key swaps its Now view, and the composer's Artifacts key opens the list. A band's key closes the list first: the story is what it asked for.
  useEffect(() => {
    if (!story) return;
    const listen = (event: KeyboardEvent) => {
      if (event.defaultPrevented || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      if (isArtifactsKey(event)) {
        event.preventDefault();
        return toggleLayer(id, "artifacts");
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
      if (band.area === "workflow") return document.querySelector<HTMLElement>('.sv-side [data-area="workflow"] select')?.focus();
      pressBand(session, band.area);
      window.setTimeout(() => document.querySelector<HTMLElement>(`.sv-side [data-area="${band.area}"] > .sv-panel__band`)?.focus(), 0);
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, [story, id, layer, session]);
  if (!story) return <>{children}{layered}{large}</>;
  // In the order a layout gives; each panel leaves itself out where a layout hides it (`SvPanel`).
  const panels: Record<string, ReactNode> = {
    workflow: <WorkflowPanel work={work} place="side" id={id} manifestId={session.manifestId} acts={draft.work} />,
    blueprint: <BlueprintPanel work={work} place="side" id={id} session={session} manifestId={session.manifestId} acts={draft.work} ed={ed} onOpenSketch={onOpenSketch} onOpenSession={onOpenSession} />,
    field: <FieldPanel session={session} repositories={repositories} place="side" live={live} />,
    landing: <LandingPanel session={session} entries={entries} place="side" id={id} />,
  };
  const right = order.map((one) => <Fragment key={one.id}>{panels[one.id]}</Fragment>);
  return (
    <>
      {children}
      {aside ? null : layered}
      {large}
      {reading}
      {createPortal(right, side)}
      {aside ? createPortal(layered, side) : null}
    </>
  );
}
