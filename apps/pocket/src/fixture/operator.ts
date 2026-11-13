// Mock only: what the fixture Gateway does with a dispatch while no Fleet is behind it. The Operator's decision is a
// stand-in; the fields of what it says are `operator.dispatched`'s.

import type { OperatorBody, OperatorDispatched } from "../gateway";

/** A live Job to fold a bug into, by Mac and repository. */
const FOLDS: Record<string, [string, string]> = {
  "studio armada": [
    "a2",
    "Redraw the Needs you band",
  ],
};

/** The first words of a send as a Job title, at most six of them. */
const titled = (words: string) => {
  const head = words.trim().split(/\s+/).slice(0, 6).join(" ");
  return head.charAt(0).toUpperCase() + head.slice(1);
};

/**
 * What the Operator does with a send, for the mock: a bug folds into a live Job of that repository where there is one, a line
 * ending in a question mark is asked, and anything else starts a Job. The kind is a hint, so an idea is not parked for being one.
 * `mac` is the Mac the Job lands on.
 */
export function decide(mac: string, id: string, send: OperatorBody): OperatorDispatched {
  const base = {
    dispatch_id: id,
    manifest_id: send.repository,
  } as const;
  if (send.text.trim().endsWith("?"))
    return {
      ...base,
      way: "asked",
    };
  const fold = send.kind === "bug" ? FOLDS[`${mac} ${send.repository}`] : undefined;
  if (fold !== undefined)
    return {
      ...base,
      way: "handed",
      session_id: fold[0],
      title: fold[1],
    };
  return {
    ...base,
    way: "started",
    session_id: mac === "laptop" ? "b1" : "a4",
    title: titled(send.text),
  };
}
