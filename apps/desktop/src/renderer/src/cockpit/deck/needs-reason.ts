// Why a Job needs him, in the words it already holds: its call's fact, else the state's own word. The Jobs row, the pillar over the
// drone and the drone's callout each say it. The mock's `?take=` picks where it is drawn (`setReasonTake`); the real app draws both.

import type { Ship, Subject } from "./ship";

export type ReasonTake = "row" | "pillar" | "both";

let take: ReasonTake = "both";

/** Mock only: the take the walk is on. Delete with `mock/main.tsx`'s read of `?take=` once one wins. */
export function setReasonTake(next: ReasonTake): void {
  take = next;
}

export const reasonOnRow = (): boolean => take !== "pillar";
export const reasonOnPillar = (): boolean => take !== "row";

/** What the Job's call says it is for, one line. */
export function reasonOf(subject: Subject, calls: Ship["calls"]): string | undefined {
  if (subject.call === undefined) return undefined;
  const said = calls.find((one) => one.key === subject.call)?.fact.replace(/\s+/g, " ").trim();
  const reason = said === undefined || said === "" ? subject.mark.said : said;
  return reason === "" ? undefined : reason;
}

/** The reason of each Job that holds a call, by the Job's key. */
export function reasonsOf(ship: Pick<Ship, "subjects" | "calls">): Map<string, string> {
  const out = new Map<string, string>();
  for (const subject of ship.subjects) {
    const reason = reasonOf(subject, ship.calls);
    if (reason !== undefined) out.set(subject.key, reason);
  }
  return out;
}
