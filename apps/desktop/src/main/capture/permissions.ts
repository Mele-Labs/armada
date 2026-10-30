// Which page may be given local network access — `docs/practices/capture-window.md`. Apart from `views.ts` so
// the popup can register its contents without pulling in the session and the tokens.

import type { WebContents } from "electron";

import { onOrigin, webOrigin } from "./address";

/** What a page is asked for: the walk's partition is shared, so the grant is by contents and not by session. */
const reach = new Map<number, (address: string) => boolean>();

/**
 * **A page's window decides who may be given it**: `allowed` is the window's own test of an origin. Forgotten
 * when the contents are destroyed, so a recycled id never inherits a grant.
 */
export function reachesLocalNetwork(contents: WebContents, allowed: (address: string) => boolean): void {
  reach.set(contents.id, allowed);
  contents.on("destroyed", () => void reach.delete(contents.id));
}

/** The test the window registered for these contents, or one that refuses every address. */
export function may(contents: WebContents | null): (address: string) => boolean {
  return (contents !== null && reach.get(contents.id)) || (() => false);
}

/**
 * A Job browser's test: **only while the owner drives, and only on the origin the page is on now.** A page the agent
 * drives is refused, since nobody chose to let it reach an app on this machine.
 */
export function whileOwnerDrives(driving: () => boolean, current: () => string): (address: string) => boolean {
  return (address) => driving() && onOrigin(webOrigin(current()) ?? "", address);
}
