// A Session's view: its thread and composer in the middle and its panels in the places round them, the same on the Cockpit and on
// the Sessions page. A Session with no `work` is drawn as it was, the thread alone with its ledger beside it.
//
// **The places.** The Status area, the Ledger and the calls keep theirs. Three places are given to the Session's own panels in
// place of what stands there with nothing selected: Activity's (left, under the list), Bays' (right) and Autopilot's (the foot).
// On the Cockpit those are real places of the deck and the panels are drawn into them (`hosts`). On the Sessions page there is
// no deck, so the same places are drawn round the thread: left of it, in the right column, and under it.
//
// **Two takes** (`take.ts`) differ in which panel goes to which place and in how the Status area draws the plan.
// A: Field left, Concerns right, Proposal in the foot, and the steps a strip over the plan.
// B: Proposal left, Field right, Concerns in the foot, and one outline with the plan hung under its step.

import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import type { LedgerEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { ConcernsPanel } from "./ConcernsPanel";
import { FieldPanel } from "./FieldPanel";
import { LedgerPanel } from "./LedgerPanel";
import { ProposalPanel } from "./ProposalPanel";
import { StatusPanel } from "./StatusPanel";
import { takeOf } from "./take";
import "./session-view.css";

/** The deck's own places, as elements: `null` before the deck has drawn one. */
export type Hosts = { left: HTMLElement | null; side: HTMLElement | null; foot: HTMLElement | null };

export function SessionWorkspace({ session, entries, repositories, hosts, children }: { session: Session; entries: readonly LedgerEntry[]; repositories: readonly RepositorySummary[]; hosts?: Hosts | undefined; children: ReactNode }) {
  const draft = useSessionsDraft();
  const work = session.work;
  if (work === undefined || draft === undefined) return <>{children}</>;
  const take = takeOf();
  const acts = draft.work;
  const { id } = session;
  const field = work.field === undefined || work.field.length === 0 ? null : <FieldPanel session={session} repositories={repositories} place={take === "a" ? "left" : "side"} />;
  const concerns =
    work.concerns === undefined || work.concerns.length === 0 ? null : (
      <ConcernsPanel concerns={work.concerns} place={take === "a" ? "side" : "foot"} onSend={(concern) => acts?.sendConcern(id, concern)} onDismiss={(concern) => acts?.dismissConcern(id, concern)} />
    );
  const proposal = work.proposal === undefined ? null : <ProposalPanel session={session} draft={draft} place={take === "a" ? "foot" : "left"} />;
  const left = take === "a" ? field : proposal;
  const side = take === "a" ? concerns : field;
  const foot = take === "a" ? proposal : concerns;
  const right = (
    <>
      <StatusPanel work={work} take={take} place="side" onApprove={() => acts?.approve(id)} />
      {side}
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
