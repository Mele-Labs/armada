// A thought typed into Helm's dock, read for where it belongs before it is asked. Draft, mock-only:
// nothing in Fleet answers the reading yet, so the window hands the dock this seam and a real
// window hands none, which leaves the dock asking Helm as it always has.

import type { ThoughtTarget } from "@armada/components";

/** The Session or Job a thought may go to. `id` is what the acts below take. */
export type RouteTarget = ThoughtTarget & { id: string };

/** What Helm made of the words: a question for itself, or a thought for other work, with the best match if any. */
export type ThoughtReading = { kind: "question" } | { kind: "thought"; match?: RouteTarget };

export type ThoughtRouting = {
  read: (text: string) => Promise<ThoughtReading>;
  /** Delivers the words: a Session gets a message, a Job a redirect to its Drone. */
  send: (target: RouteTarget, text: string) => void;
  /** Opens the Session or Job. */
  open: (target: RouteTarget) => void;
  /** Opens the Dispatch panel holding the words, with this kind picked. */
  startNew: (kind: "session" | "job", text: string) => void;
};
