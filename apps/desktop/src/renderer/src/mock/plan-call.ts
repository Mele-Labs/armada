// The waiting items Fleet serves for a page and a plan. A page he has not approved waits as `walk:<url>`, source `walk`, with the window's
// title, until his Approve: a window still being changed raises none, and closing the window keeps it. A pending plan waits as
// `plan:<version>`, source `plan`, only when nothing else is held back: not while a page is being changed, and not while a walk waits.

import type { WaitingItem } from "@armada/protocol";

/** `version` is the plan's own; the mock plans that carry none are version 1. */
export const planCall = (version: number | undefined, since: string): WaitingItem => ({ id: `plan:${version ?? 1}`, text: "Plan to approve", since, source: "plan" });

/** What he is owed besides the plan's call: approving settles it, and so does recording the plan again before it is asked. */
export const withoutPlanCall = (waiting: readonly WaitingItem[] | undefined): WaitingItem[] => (waiting ?? []).filter((one) => one.source !== "plan");

/** The page at `url`, waiting for his Approve. Its window being open or closed changes nothing. */
export const walkCall = (url: string, title: string, since: string): WaitingItem => ({ id: `walk:${url}`, text: title, since, source: "walk", act: { kind: "walk", target: url } });

/** The same list with the page at `url` put to him, or taken off it. */
export const withWalkCall = (waiting: readonly WaitingItem[] | undefined, url: string, call?: WaitingItem): WaitingItem[] => [...(waiting ?? []).filter((one) => one.id !== `walk:${url}`), ...(call === undefined ? [] : [call])];
