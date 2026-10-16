// A Session's view: its thread and composer in the middle and its panels in the places round them, the same on the Cockpit and on
// the Sessions page. A Session with no `work` is drawn as it was, the thread alone with its ledger beside it.
//
// **The places.** The Status area, the Ledger and Incoming calls keep theirs, and the Ledger keeps the size it has with nothing
// selected. Two places are given to the Session's own panels in place of what stands there with nothing selected: Activity's (left,
// under the list) and Autopilot's (the foot, where Dispatch controls also give way). The proposal has no panel: the title is the Session's name in its header, the model and the effort are the composer's dropdowns marked as
// picked by the proposer, and the workflow is a picker over the steps in Status. On the Cockpit those places are the deck's own and the panels are drawn into them (`hosts`). On the Sessions
// page there is no deck, so the same places are drawn round the thread: left of it, in the right column, and under it.
//
// **Two takes** (`take.ts`) differ in where Concerns stand and in how the Status area draws the plan.
// A: Concerns as cards in Incoming calls, and the steps in a list of their own with the plan a section under it.
// B: Concerns in the foot, in Autopilot's place, and one outline with the plan hung under its step.
// The Field is on the left in both. The Sessions page has no Incoming calls, so A draws its Concerns in the foot there too.

import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import type { LedgerEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { ConcernsPanel } from "./ConcernsPanel";
import { FieldPanel } from "./FieldPanel";
import { LedgerPanel } from "./LedgerPanel";
import { StatusPanel } from "./StatusPanel";
import { takeOf } from "./take";
import "./session-view.css";

/** The deck's own places, as elements: `null` before the deck has drawn one. */
export type Hosts = { left: HTMLElement | null; side: HTMLElement | null; foot: HTMLElement | null; calls: HTMLElement | null };

export function SessionWorkspace({ session, entries, repositories, hosts, children }: { session: Session; entries: readonly LedgerEntry[]; repositories: readonly RepositorySummary[]; hosts?: Hosts | undefined; children: ReactNode }) {
  const draft = useSessionsDraft();
  const work = session.work;
  if (work === undefined || draft === undefined) return <>{children}</>;
  const take = takeOf();
  const acts = draft.work;
  const { id } = session;
  const field = work.field === undefined || work.field.length === 0 ? null : <FieldPanel session={session} repositories={repositories} place="left" />;
  const concern = (place: "calls" | "foot") =>
    work.concerns === undefined || work.concerns.length === 0 ? null : (
      <ConcernsPanel concerns={work.concerns} place={place} onSend={(one) => acts?.sendConcern(id, one)} onDismiss={(one) => acts?.dismissConcern(id, one)} />
    );
  const foot = concern("foot");
  const right = (
    <>
      <StatusPanel work={work} take={take} place="side" id={id} acts={acts} onStart={() => acts?.start(id)} onApprove={() => acts?.approve(id)} />
      <LedgerPanel session={session} entries={entries} place="side" />
    </>
  );
  if (hosts !== undefined) {
    return (
      <>
        {children}
        {hosts.left === null || field === null ? null : createPortal(field, hosts.left)}
        {hosts.side === null ? null : createPortal(right, hosts.side)}
        {take === "a" ? (hosts.calls === null ? null : createPortal(concern("calls"), hosts.calls)) : hosts.foot === null ? null : createPortal(concern("foot"), hosts.foot)}
      </>
    );
  }
  return (
    <div className="sv" data-take={take}>
      {field === null ? null : <div className="sv__left">{field}</div>}
      <div className="sv__centre">
        <div className="sv__thread">{children}</div>
        {foot === null ? null : <div className="sv__foot">{foot}</div>}
      </div>
      <div className="sv__right">{right}</div>
    </div>
  );
}
