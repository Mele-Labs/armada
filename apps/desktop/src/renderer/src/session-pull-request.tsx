// What the side panel draws of a Session's pull request beyond the forge's read: its description without a template's
// comments, the failing checks' names, and the message that sends them back to the Session.

import { useCallback, useState } from "react";
import type { SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";

type PullRequest = Extract<SessionAttachment, { kind: "pull_request" }>;

/** A description without the HTML comments a forge's template leaves for its author: Prose would draw them as characters. */
export const promptsOut = (body: string | undefined): string => (body ?? "").replace(/<!-{2}[\s\S]*?-{2}>/g, "").trim();

/** The failing checks' names: Fleet serves them as one string, joined with a comma and a space. */
export const failingOf = (one: PullRequest): string[] =>
  (one.checks.state === "failed" ? one.checks.failing : (one.checks.red ?? "")).split(", ").filter((name) => name !== "");

const keyOf = (sessionId: string, one: PullRequest) => `${sessionId}#${one.number}:${failingOf(one).join(",")}`;

/**
 * Tells a Session its pull request's checks failed, as a message it is sent the way the composer's
 * is. **Kept per pull request and per set of failing checks**, so a second failure asks again.
 */
export function useSendBack(sessionId: string, send: SessionsDraft["send"] | undefined) {
  const [sent, setSent] = useState<ReadonlySet<string>>(new Set());
  const sendBack = useCallback(
    (one: PullRequest) => {
      const names = failingOf(one);
      const text = names.length === 0 ? `Checks failed on #${one.number}` : `Checks failed on #${one.number}: ${names.join(", ")}`;
      const key = keyOf(sessionId, one);
      void Promise.resolve(send?.(sessionId, { text, files: [], sketches: [], tags: [] })).then((went) => {
        if (went !== false) setSent((was) => new Set(was).add(key));
      });
    },
    [sessionId, send],
  );
  return { sentBack: (one: PullRequest) => sent.has(keyOf(sessionId, one)), sendBack };
}
