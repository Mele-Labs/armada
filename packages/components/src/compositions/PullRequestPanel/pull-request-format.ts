import type { WritePullRequest } from "@armada/protocol";

import { ageOf } from "../../time-format";

// The words and rules of the pull request panel that are not drawing: how long ago, how long a
// check ran, and which writes the comment box may send.

/** `12m ago`, `2h ago`, `3d ago`, as every age is worded. Absent where the instant will not parse. */
export function agoOf(at: string | undefined, now: number = Date.now()): string | undefined {
  const then = at === undefined ? Number.NaN : Date.parse(at);
  return Number.isNaN(then) ? undefined : ageOf(then, now);
}

/** `45s`, `2m 11s`, `1h 03m`: the span a check ran, written as the Job board writes one. */
export function ranFor(seconds: number): string {
  const whole = Math.max(0, Math.round(seconds));
  if (whole < 60) return `${whole}s`;
  const minutes = Math.floor(whole / 60);
  if (minutes < 60) return `${minutes}m ${String(whole % 60).padStart(2, "0")}s`;
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

/** Whether the comment box may send: a comment and a request for changes need words, an approval need not. */
export function mayWrite(kind: WritePullRequest["kind"], body: string): boolean {
  return kind === "approve" || body.trim() !== "";
}
