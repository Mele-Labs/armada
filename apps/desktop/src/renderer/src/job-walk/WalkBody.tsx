// The tab of one walk: its state is kept in `store.ts`, so leaving the Job and coming back, or moving the tab to another pane, finds the page, take and step as they were.

import { useEffect } from "react";
import type { Session } from "@armada/screens/src/draft/sessions";

import { focusKind } from "../job-panes/focus";
import { rearrange, useJobPanes } from "../job-panes/store";
import { closeTab, panesOf, type Tab } from "../job-panes/tree";
import { useSendHeld } from "./held";
import { mockPage } from "./mock-pages";
import {
  approveWalk,
  goToStep,
  holdNote,
  markSeen,
  pickTake,
  useHeldNotes,
  useWalkState,
} from "./store";
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
  const notes = useHeldNotes(jobId);
  const send = useSendHeld(session);
  // The thread beside the walk is in front where its pane shows it; the status brings it when it is not.
  const { arrangement } = useJobPanes(jobId);
  const threadFront = panesOf(arrangement.root).some(
    (pane) => pane.tabs.find((one) => one.id === pane.active)?.kind === "thread",
  );
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
      aim={`Onto ${session.title ?? session.id}`}
      onNote={(said, on) => holdNote(jobId, said, on)}
      onApprove={() => approveWalkTab(jobId, tab)}
      {...(notes.length === 0
        ? {}
        : {
            held: {
              notes: notes.length === 1 ? ("one" as const) : ("several" as const),
              onSendAndApprove: () => void send().then(() => approveWalkTab(jobId, tab)),
            },
          })}
      panel={{
        kind: "session",
        open: threadFront,
        onToggle: () => focusKind(jobId, "thread"),
      }}
      {...(state.readiness === undefined
        ? {}
        : {
            readiness: state.readiness,
          })}
      page={mockPage(walk.title, take)}
    />
  );
}
