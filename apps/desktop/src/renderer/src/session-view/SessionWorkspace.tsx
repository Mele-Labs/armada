// A Session's view: its thread and composer in the middle and its panels in the places round them on the Cockpit, the one place a
// Session is viewed. A Session with no `work` is drawn as it was, the thread alone with its ledger beside it.
//
// **The places.** Jobs and Sessions, Flight control and the Ledger keep theirs. Incoming calls stands on the left under the list, without the
// selected Session's own calls: what needs him for it is a card in its thread. With nothing selected Activity takes the place it leaves on the right. Three places
// are given to the Session's own panels: the right column holds Status, the Ledger and the Plan, where Incoming calls used to be, the
// foot holds the Field from the moment the Session is selected, and Dispatch controls give way. The proposal has no panel: the title
// is the Session's name in its header, the model and the effort are the composer's dropdowns marked as picked by the proposer, and
// the workflow is a picker over the steps in Status. The panels are drawn into the deck's own places (`hosts`).

import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import type { LedgerEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { FieldPanel } from "./FieldPanel";
import { LedgerPanel } from "./LedgerPanel";
import type { PlanEditing } from "./PlanEdit";
import { PlanPanel } from "./PlanPanel";
import { StatusPanel } from "./StatusPanel";
import "./session-view.css";

/** The deck's own places, as elements: `null` before the deck has drawn one. */
export type Hosts = { side: HTMLElement | null; foot: HTMLElement | null };

export function SessionWorkspace({ session, entries, repositories, hosts, ed, onOpenSketch, children }: { session: Session; ed: PlanEditing | undefined; entries: readonly LedgerEntry[]; repositories: readonly RepositorySummary[]; hosts?: Hosts | undefined; onOpenSketch: (sketch: string) => void; children: ReactNode }) {
  const draft = useSessionsDraft();
  const work = session.work;
  if (work === undefined || draft === undefined || hosts === undefined) return <>{children}</>;
  const acts = draft.work;
  const { id } = session;
  const right = (
    <>
      {work.railOff === true ? null : <StatusPanel work={work} place="side" id={id} manifestId={session.manifestId} acts={acts} />}
      <LedgerPanel session={session} entries={entries} place="side" />
      {work.plan === undefined || ed === undefined ? null : <PlanPanel id={id} plan={work.plan} place="side" ed={ed} onOpenSketch={onOpenSketch} />}
    </>
  );
  return (
    <>
      {children}
      {hosts.side === null ? null : createPortal(right, hosts.side)}
      {hosts.foot === null ? null : createPortal(<FieldPanel session={session} repositories={repositories} place="foot" />, hosts.foot)}
    </>
  );
}
