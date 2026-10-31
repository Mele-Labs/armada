// Which page may be given local network access — `docs/practices/capture-window.md`. Apart from `views.ts` so
// the popup can register its contents without pulling in the session and the tokens.

import type { WebContents } from "electron";

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

/** What a Job browser's page decides for itself: its own saved choices instead of the window's origin test. */
export type Decider = {
  request(permission: string, requestingUrl: string, details: { mediaTypes?: readonly string[] }): Promise<boolean>;
  check(permission: string, requestingOrigin: string, details: { mediaTypes?: readonly string[] }): boolean;
  download(event: { preventDefault(): void }, url: string, redownload: () => void): void;
};

const deciders = new Map<number, Decider>();

/** Forgotten when the contents are destroyed, as `reach` is. */
export function decidesPermissions(contents: WebContents, decider: Decider): void {
  deciders.set(contents.id, decider);
  contents.on("destroyed", () => void deciders.delete(contents.id));
}

export function deciderOf(contents: WebContents | null): Decider | undefined {
  return contents === null ? undefined : deciders.get(contents.id);
}
