// A Session's view: its thread and composer in the middle, and little else at a glance. A Session with no `work` is drawn as it was, the thread
// alone with its ledger beside it.
//
// **What stays on screen.** The Work list, the thread and composer, the Field and Progress in the right column, and Flight control. Progress is
// Status and the Plan as one panel. The rest is a key away: Flight control opens a layer over the stage for Artifacts (everything the Session
// made or used), Subagents, Checks and Slot, and Back returns to the thread as it was. Pull requests are the Merge line's. The slot and branch
// are drawn only when something is wrong with them: as a call in the thread, and by lighting the Slot key. The panels are drawn into the deck's
// right column (`hosts`).

import { useEffect } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import type { LedgerEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { FieldLayer } from "./FieldLayer";
import { FieldPanel } from "./FieldPanel";
import { dropLayer, useLayer } from "./layers";
import type { PlanEditing } from "./PlanEdit";
import { ProgressPanel } from "./ProgressPanel";
import { WorkflowPanel } from "./WorkflowPanel";
import type { LiveOpen } from "./TurnAccountBlock";
import { Layer } from "./SessionLayers";
import type { SessionBays } from "./slot-trouble";
import "./session-view.css";

/** The deck's own place for a Session's panels, as an element: `null` before the deck has drawn it. */
export type Hosts = { side: HTMLElement | null };

export function SessionWorkspace({ session, entries, repositories, hosts, ed, bays, onOpenSketch, onOpenSession, onOpenCheck, live, children }: { session: Session; ed: PlanEditing | undefined; entries: readonly LedgerEntry[]; repositories: readonly RepositorySummary[]; hosts?: Hosts | undefined; bays: SessionBays | undefined; onOpenSketch: (sketch: string, at?: string) => void; onOpenSession: (id: string) => void; onOpenCheck: (run: number) => void; live?: LiveOpen; children: ReactNode }) {
  const draft = useSessionsDraft();
  const { id } = session;
  const open = useLayer(id);
  // The Field large draws itself (`FieldLayer.tsx`); the other layers are `Layer`'s.
  const layer = open === "field" ? undefined : open;
  const large = open === "field" ? <FieldLayer session={session} repositories={repositories} /> : null;
  // A layer belongs to the Session it was opened over.
  useEffect(() => () => dropLayer(id), [id]);
  const work = session.work;
  const layered = layer === undefined ? null : <Layer id={layer} session={session} entries={entries} bays={bays} onOpenCheck={onOpenCheck} onOpenSession={onOpenSession} onOpenSketch={onOpenSketch} />;
  if (work === undefined || draft === undefined || hosts === undefined) return <>{children}{layered}{large}</>;
  const right = (
    <>
      <FieldPanel session={session} repositories={repositories} place="side" />
      <WorkflowPanel work={work} place="side" id={id} manifestId={session.manifestId} acts={draft.work} />
      <ProgressPanel work={work} place="side" id={id} session={session} manifestId={session.manifestId} acts={draft.work} ed={ed} onOpenSketch={onOpenSketch} onOpenSession={onOpenSession} live={live} />
    </>
  );
  return (
    <>
      {children}
      {layered}
      {large}
      {hosts.side === null ? null : createPortal(right, hosts.side)}
    </>
  );
}
