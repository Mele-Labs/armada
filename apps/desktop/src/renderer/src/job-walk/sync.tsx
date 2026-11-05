// Gives each Job the tabs of the walks it owes him, and takes a tab away once its walk is approved. It runs for every Job, the one he is in or not, so a walk that arrives while he is
// elsewhere is waiting in its Job's panes when he opens it. **Nothing moves on his screen for a Job he is not in**: the arrangement changes, and no window opens.

import { owesOf } from "@armada/screens/src/draft/sessions";
import { useEffect } from "react";

import { useSessions } from "../sessions-draft";
import { useDeck } from "../cockpit/deck/store";
import { look, peek, rearrange } from "../job-panes/store";
import {
  activate,
  arriveWalk,
  closeTab,
  panesOf,
  reopenThread,
  type Arrangement,
} from "../job-panes/tree";
import { arrivedBefore, pressedWalks, settlePressed, useWalksVersion } from "./store";
import { owedWalks, type JobWalk } from "./walks";

/**
 * `here`: the Cockpit is what is on screen. **A Job opens as he left it**: a walk arrives behind the tab in front and pulses until he opens it, and only pressing it brings it forward.
 * A thread he closed comes back where it stood when a walk arrives, and when he opens a Job that waits on him, since what waits on him is drawn there.
 */
export function WalkTabsSync({ here }: { here: boolean }) {
  const sessions = useSessions();
  const walks = useWalksVersion();
  const opened = useDeck((deck) => deck.sel);
  const inJob =
    here && opened?.kind === "session" ? opened.key.replace(/^session:/, "") : undefined;
  const arrange = (id: string, walk: JobWalk, behind: boolean) => {
    look(id, (now: Arrangement) => arriveWalk(reopenThread(now), walk.title, walk.id, behind));
  };
  const tabsOf = (id: string) =>
    panesOf(peek(id).root).flatMap((pane) => pane.tabs.filter((tab) => tab.kind === "walk"));
  useEffect(() => {
    for (const session of sessions) {
      if (session.dead !== undefined) continue;
      const owed = owedWalks(session);
      const tabs = tabsOf(session.id);
      // A walk settled from anywhere takes its tab with it.
      for (const tab of tabs)
        if (!owed.some((one) => one.id === tab.page))
          rearrange(session.id, (arrangement) => closeTab(arrangement, tab.id));
      // A walk gets its tab once. One he closed with its × is not opened again by a change in the walks.
      for (const walk of owed)
        if (!tabs.some((tab) => tab.page === walk.id) && !arrivedBefore(session.id, walk.id))
          arrange(session.id, walk, true);
    }
  }, [
    sessions,
    walks,
  ]);
  // A walk pressed open comes to the front, in the pane that holds it or in a tab made for it. It waits here for its Job to be one the sessions hold.
  useEffect(() => {
    for (const one of pressedWalks()) {
      const session = sessions.find((session) => session.id === one.jobId);
      if (session === undefined) continue;
      settlePressed(one.jobId, one.walk);
      const tab = tabsOf(one.jobId).find((tab) => tab.page === one.walk);
      if (tab !== undefined) look(one.jobId, (arrangement) => activate(arrangement, tab.id));
      else {
        const walk = owedWalks(session).find((owed) => owed.id === one.walk);
        if (walk !== undefined) arrange(one.jobId, walk, false);
      }
    }
  }, [
    sessions,
    walks,
  ]);
  // Opening a Job brings back the thread he closed where it waits on him, and the tab of a walk he closed and has not approved, behind what he left in front.
  useEffect(() => {
    const session = sessions.find((one) => one.id === inJob);
    if (session === undefined) return;
    if (owesOf(session).length > 0) look(session.id, reopenThread);
    for (const walk of owedWalks(session))
      if (!tabsOf(session.id).some((tab) => tab.page === walk.id)) arrange(session.id, walk, true);
  }, [
    opened,
  ]);
  return null;
}
