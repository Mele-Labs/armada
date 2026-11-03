// Pressing a Job's walk open, from Needs you, the Job's own rows or the palette: the Job opens in the Cockpit and its walk's tab comes to the front. There is no window of the walk's own.

import { askToOpenSession } from "../open-session";
import { bringWalkForward } from "./store";

/** `walk` is the page's address, which names the walk. */
export function openJobWalk(jobId: string, walk: string): void {
  bringWalkForward(jobId, walk);
  askToOpenSession(jobId);
}
