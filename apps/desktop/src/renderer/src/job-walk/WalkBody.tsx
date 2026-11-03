// The tab of one walk: its state is kept in `store.ts`, so leaving the Job and coming back, or moving the tab to another pane, finds the page, take and step as they were.

import { useEffect } from "react";
import type { Session } from "@armada/screens/src/draft/sessions";

import { rearrange } from "../job-panes/store";
import { closeTab, type Tab } from "../job-panes/tree";
import { mockPage } from "./mock-pages";
import { approveWalk, goToStep, markSeen, pickTake, useWalkState } from "./store";
import { walksOf } from "./walks";
import { WalkView } from "./WalkView";

/** Approving settles the walk and closes its tab. */
export function approveWalkTab(jobId: string, tab: Tab): void {
  if (tab.page !== undefined) approveWalk(jobId, tab.page);
  rearrange(jobId, (arrangement) => closeTab(arrangement, tab.id));
}

export function WalkBody({ session, tab }: { session: Session; tab: Tab }) {
  const jobId = session.id;
  const walk = walksOf(session).find((one) => one.id === tab.page);
  const state = useWalkState(jobId, tab.page);
  // Drawn in front of him: the tab stops pulsing.
  useEffect(() => {
    if (tab.page !== undefined) markSeen(jobId, tab.page);
  }, [
    jobId,
    tab.page,
  ]);
  if (walk === undefined) return null;
  const take = walk.takes.length === 0 ? undefined : walk.takes[state.take];
  return (
    <WalkView
      name={walk.title}
      takes={walk.takes}
      take={state.take}
      onTake={(at) => pickTake(jobId, walk.id, at)}
      steps={walk.steps}
      step={state.step}
      onStep={(to) => goToStep(jobId, walk.id, to)}
      noteKey={`${jobId}:${walk.id}`}
      // Mock only: a note goes nowhere yet.
      onNote={() => {}}
      onApprove={() => approveWalkTab(jobId, tab)}
      page={mockPage(walk.title, take)}
    />
  );
}
