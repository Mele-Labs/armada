// The redirects this window sent, held by the engine a Session's own messages are (`outgoing.ts`), so a redirect goes
// through the same states: going, refused with a Retry, then delivered. One per Job; sending again replaces it.

import type { SentRedirect, SentRedirects } from "@armada/jobs";

import type { SessionRow } from "@armada/screens/src/draft/sessions";

import { outgoing } from "./outgoing";
import type { Held } from "./outgoing";

const engine = outgoing();
const latest = new Map<string, Held>();

const none = new Set<string>();
/** One array, so the engine's cache holds: a new one each read would make every read a new row. */
const NO_ROWS: readonly SessionRow[] = [];

const rowOf = (jobId: string): (SentRedirect & { kind: string }) | undefined => {
  const row = engine.overlay(jobId, NO_ROWS)[0];
  return row?.kind === "message" ? row : undefined;
};

export const sentRedirects: SentRedirects & {
  /** Holds the redirect and delivers it with `how`, to whether Fleet took it. */
  send: (jobId: string, text: string, how: () => Promise<boolean>) => Promise<boolean>;
} = {
  // The engine hands back the same row until the redirect changes, which is what a store read by a hook needs.
  get: (jobId) => rowOf(jobId),
  subscribe: engine.subscribe,
  retry: (jobId) => {
    const row = rowOf(jobId);
    if (row !== undefined) engine.retry(jobId, row.id, none);
  },
  send: (jobId, text, how) => {
    latest.get(jobId)?.drop();
    const held = engine.hold(jobId, { text, files: [], sketches: [], tags: [] }, none);
    latest.set(jobId, held);
    return held.deliver(how);
  },
};
