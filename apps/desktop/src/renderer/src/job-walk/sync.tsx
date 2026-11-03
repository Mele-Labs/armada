// Gives each Job the tabs of the walks it owes him, and takes a tab away once its walk is approved. It runs for every Job, the one he is in or not, so a walk that arrives while he is
// elsewhere is waiting in its Job's panes when he opens it. **Nothing moves on his screen for a Job he is not in**: the arrangement changes, and no window opens.

import { useEffect } from "react";

import { useSessions } from "../sessions-draft";
import { useDeck } from "../cockpit/deck/store";
import { look, peek, rearrange } from "../job-panes/store";
import { arriveWalk, closeTab, hasKind, panesOf, type Arrangement } from "../job-panes/tree";
import { arrivedBefore, useWalksVersion } from "./store";
import { owedWalks } from "./walks";

/**
 * `here`: the Cockpit is what is on screen. A walk comes to the front of its pane only for a Job he is not in; in the Job he is in it arrives behind
 * the tab in front, and pulses until he opens it.
 */
export function WalkTabsSync({ here }: { here: boolean }) {
  const sessions = useSessions();
  const walks = useWalksVersion();
  const opened = useDeck((deck) => deck.sel);
  const inJob =
    here && opened?.kind === "session" ? opened.key.replace(/^session:/, "") : undefined;
  useEffect(() => {
    for (const session of sessions) {
      if (session.dead !== undefined) continue;
      const owed = owedWalks(session);
      const tabs = panesOf(peek(session.id).root).flatMap((pane) =>
        pane.tabs.filter((tab) => tab.kind === "walk"),
      );
      // A walk settled from anywhere takes its tab with it.
      for (const tab of tabs)
        if (!owed.some((one) => one.id === tab.page))
          rearrange(session.id, (arrangement) => closeTab(arrangement, tab.id));
      for (const walk of owed) {
        if (tabs.some((tab) => tab.page === walk.id) || arrivedBefore(session.id, walk.id))
          continue;
        const behind = session.id === inJob;
        const change = (now: Arrangement) => arriveWalk(now, walk.title, walk.id, behind);
        // A tab that joins a pane is content, and one that makes a split is arrangement.
        if (hasKind(peek(session.id), "walk")) look(session.id, change);
        else rearrange(session.id, change, undefined, true);
      }
    }
  }, [
    sessions,
    walks,
    inJob,
  ]);
  return null;
}
