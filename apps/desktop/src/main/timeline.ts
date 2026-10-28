// What happened across the Fleet, as main holds it: one request to Fleet for a window and the stream's
// `timeline.appended`, passed on to every window. The newest `KEPT` entries are held, and kept on disk by `remembered.ts`,
// so a window opened before Fleet answers draws the last ones; each window holds what it drew.

import type { FleetTimeline, Outcome, TimelineAsk, TimelineEvent } from "@armada/protocol";
import type { FleetTimelineRead } from "../shared/api/timeline";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** Entries held and kept on disk, the newest. */
const KEPT = 1000;

/** The query string for a window: only what was asked, each encoded. */
export function timelinePath(window: TimelineAsk): string {
  const query = [
    ...(window.since === undefined ? [] : [`since=${encodeURIComponent(window.since)}`]),
    ...(window.until === undefined ? [] : [`until=${encodeURIComponent(window.until)}`]),
    ...(window.limit === undefined ? [] : [`limit=${window.limit}`]),
  ];
  return query.length === 0 ? "/timeline" : `/timeline?${query.join("&")}`;
}

export class TimelineHost {
  private readonly port: () => number | null;
  private told: (event: TimelineEvent) => void = () => {};
  private held: readonly TimelineEvent[] = [];
  private keep: (events: TimelineEvent[]) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where an appended entry goes: main's, which owns the windows. */
  onAppended(told: (event: TimelineEvent) => void): void {
    this.told = told;
  }

  /** Start from the entries on disk, and say where the entries held are kept. */
  remembering(held: readonly TimelineEvent[], keep: (events: TimelineEvent[]) => void): void {
    this.held = held;
    this.keep = keep;
  }

  /** Fleet numbers an entry as it writes it, so a window read and an append that overlap hold it once. */
  private merge(more: readonly TimelineEvent[]): void {
    const seen = new Map(this.held.map((one) => [one.id, one]));
    for (const one of more) seen.set(one.id, one);
    const kept = [...seen.values()].sort((a, b) => a.id - b.id).slice(-KEPT);
    this.held = kept;
    this.keep(kept);
  }

  /** `timeline.appended`. */
  appended(event: TimelineEvent): void {
    this.merge([event]);
    this.told(event);
  }

  async get(window: TimelineAsk): Promise<FleetTimelineRead> {
    const port = this.port();
    if (port === null) return this.held.length === 0 ? { ok: false, outcome: NOT_CONNECTED } : { ok: true, value: { events: [...this.held] } };
    const answer = await ask(port, "GET", timelinePath(window));
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    const value = answer.body as FleetTimeline;
    this.merge(value.events);
    return { ok: true, value };
  }
}
