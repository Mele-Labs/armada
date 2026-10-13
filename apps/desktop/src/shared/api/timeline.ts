// What happened across the Fleet, for the Cockpit's timeline: a window read once, and the event that
// carries each new entry. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts`
// compose them.

import type { FleetTimeline, Outcome, TimelineAsk, TimelineEvent } from "@armada/protocol";

/** What reading the window came to: Fleet's entries, oldest first, or the refusal to word. */
export type FleetTimelineRead = { ok: true; value: FleetTimeline } | { ok: false; outcome: Outcome };

export type TimelineApi = {
  /** The entries in a window, as `GET /timeline` answers them. A Fleet before protocol 23.82 answers a refusal. */
  getFleetTimeline: (ask?: TimelineAsk) => Promise<FleetTimelineRead>;
  /** `timeline.appended`: the entry Fleet just kept. */
  onTimelineAppended: (onAppended: (event: TimelineEvent) => void) => () => void;
};

export const TIMELINE_CHANNELS = {
  getFleetTimeline: "bridge:get-fleet-timeline",
  /** Main to every window, not a request. */
  timelineAppended: "bridge:timeline-appended",
} as const;
