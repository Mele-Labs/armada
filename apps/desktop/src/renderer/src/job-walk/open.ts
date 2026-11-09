// Pressing a Job's walk open, from Needs you, the Job's own rows or the palette: the Job opens in the Cockpit and its walk's tab comes to the front. There is no window of the walk's own.
// A native walk is the exception: the press opens the native app's window at once, and the tab, with its Open and the reason, comes forward only when Fleet does not.

import { askToOpenSession } from "../open-session";
import { bringWalkForward } from "./store";

/** A walk of a native screen (`show_window` on `armada://walk/<name>`), which Fleet opens from the Job's slot. */
const NATIVE_WALK = "armada://walk/";

/** `walk` is the page's address, which names the walk. */
export function openJobWalk(jobId: string, walk: string): void {
  const openNative = window.armada?.jobWalk?.openNative;
  if (walk.startsWith(NATIVE_WALK) && openNative !== undefined) {
    void openNative(jobId, walk).then((outcome) => {
      if (!outcome.ok) showTab(jobId, walk);
    });
    return;
  }
  showTab(jobId, walk);
}

function showTab(jobId: string, walk: string): void {
  bringWalkForward(jobId, walk);
  askToOpenSession(jobId);
}
