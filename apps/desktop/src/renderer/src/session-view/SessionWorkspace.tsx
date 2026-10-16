// A Session's view: its thread and composer in the middle and its panels in the places round them, the same on the Cockpit and on
// the Sessions page. A Session with no `work` is drawn as it was, the thread alone with its ledger beside it.
//
// **The places.** The Status area, the Ledger and Incoming calls keep theirs, and the Ledger keeps the size it has with nothing
// selected. Two places are given to the Session's own panels in place of what stands there with nothing selected: Activity's (left,
// under the list) and Autopilot's (the foot, where Dispatch controls also give way). The proposal has no panel: the title is the Session's name in its header, the model and the effort are the composer's dropdowns marked as
// picked by the proposer, and the workflow is a picker over the steps in Status. On the Cockpit those places are the deck's own and the panels are drawn into them (`hosts`). On the Sessions
// page there is no deck, so the same places are drawn round the thread: left of it, in the right column, and under it.
//
// **Two takes** (`take.ts`) place the Plan and the Field in the two places that give way, the other way about. A: the Field in
// Activity's place (left) and the Plan in Autopilot's (the foot). B: the Plan left and the Field in the foot. Judge concerns are cards
// in Incoming calls in both. The Sessions page has no Incoming calls, so its concerns are not drawn there.

import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import type { LedgerEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { ConcernsPanel } from "./ConcernsPanel";
import { FieldPanel } from "./FieldPanel";
import { LedgerPanel } from "./LedgerPanel";
import { PlanPanel } from "./PlanPanel";
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
  // The two takes put Plan and the Field in the two places that give way on the left and in the foot, the other way about.
  const [fieldAt, planAt] = take === "a" ? (["left", "foot"] as const) : (["foot", "left"] as const);
  const field = work.field === undefined || work.field.length === 0 ? null : <FieldPanel session={session} repositories={repositories} place={fieldAt} />;
  const plan = work.plan === undefined ? null : <PlanPanel plan={work.plan} place={planAt} onApprove={() => acts?.approve(id)} />;
  const concerns =
    work.concerns === undefined || work.concerns.length === 0 ? null : (
      <ConcernsPanel concerns={work.concerns} place="calls" onSend={(one) => acts?.sendConcern(id, one)} onDismiss={(one) => acts?.dismissConcern(id, one)} />
    );
  const left = fieldAt === "left" ? field : plan;
  const foot = fieldAt === "foot" ? field : plan;
  const right = (
    <>
      <StatusPanel work={work} place="side" id={id} acts={acts} onStart={() => acts?.start(id)} />
      <LedgerPanel session={session} entries={entries} place="side" />
    </>
  );
  if (hosts !== undefined) {
    return (
      <>
        {children}
        {hosts.left === null || left === null ? null : createPortal(left, hosts.left)}
        {hosts.side === null ? null : createPortal(right, hosts.side)}
        {hosts.foot === null || foot === null ? null : createPortal(foot, hosts.foot)}
        {hosts.calls === null ? null : createPortal(concerns, hosts.calls)}
      </>
    );
  }
  return (
    <div className="sv" data-take={take}>
      {left === null ? null : <div className="sv__left">{left}</div>}
      <div className="sv__centre">
        <div className="sv__thread">{children}</div>
        {foot === null ? null : <div className="sv__foot">{foot}</div>}
      </div>
      <div className="sv__right">{right}</div>
    </div>
  );
}
