// The redirect this window sent a Job's Drone, as the message box under the Drone's log draws it: the same row a
// Session's thread draws for the person's own words, going, refused with a Retry, or delivered.
//
// **The host owns the store**, since the send is its call to Fleet (`commands.redirect`), and hands it down by
// context so the two boxes that read it need no prop through every tab. One redirect per Job: sending again
// replaces what was outstanding, as it does on Fleet. A window that did not send one has none here, and the box
// falls back to Fleet's own `redirecting` sentence (`steering.ts`).

import { createContext, useContext, useSyncExternalStore } from "react";

export type SentRedirect = { id: string; at: string; text: string; delivery?: "sending" | "failed" };

export type SentRedirects = {
  get: (jobId: string) => SentRedirect | undefined;
  subscribe: (listener: () => void) => () => void;
  /** Sends the redirect that did not go again. */
  retry: (jobId: string) => void;
};

export const SentRedirectsContext = createContext<SentRedirects | undefined>(undefined);

const NEVER = () => () => undefined;
const NONE = () => undefined;

/** What the message box under a Job's Drone is given for the redirect sent: the row, and the Retry on it. */
export function useSentRedirect(jobId: string): { sent?: SentRedirect; onRetry?: () => void } {
  const held = useContext(SentRedirectsContext);
  const sent = useSyncExternalStore(
    held?.subscribe ?? NEVER,
    held === undefined ? NONE : () => held.get(jobId),
    // Drawn on a server, as the fixtures' test does, nothing has been sent.
    NONE,
  );
  if (held === undefined || sent === undefined) return {};
  return { sent, onRetry: () => held.retry(jobId) };
}
