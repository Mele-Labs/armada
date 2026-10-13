// What happened across the Fleet, as main holds it: one request to Fleet for a window and the stream's
// `timeline.appended`, passed on to every window. Nothing is kept here; each window holds what it drew.

import type { FleetTimeline, Outcome, TimelineAsk, TimelineEvent } from "@armada/protocol";
import type { FleetTimelineRead } from "../shared/api/timeline";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

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

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where an appended entry goes: main's, which owns the windows. */
  onAppended(told: (event: TimelineEvent) => void): void {
    this.told = told;
  }

  /** `timeline.appended`. */
  appended(event: TimelineEvent): void {
    this.told(event);
  }

  async get(window: TimelineAsk): Promise<FleetTimelineRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", timelinePath(window));
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as FleetTimeline };
  }
}
