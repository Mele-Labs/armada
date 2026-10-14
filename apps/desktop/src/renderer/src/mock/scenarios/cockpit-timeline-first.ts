// The Cockpit opening on its timeline, with Autopilot's bar at the top and no dispatch bar. `fleet-timeline`'s
// two hours of events, and a pull request on a Session's branch beside the one a Job opened: each is a set of
// marks on the track of whoever owns it, and the pull requests nobody owns (#1893, #1894, #1891, #1895,
// #1889) have no row. The walk `cockpit-bar-out-timeline-first` plays it.

import type { TimelineEvent } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s207FleetTimeline as base } from "./fleet-timeline";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** Pull request #1896, held by the Session `Flaky store test` through the branch it works on (`fix/flaky-store`). */
const PULL = { kind: "pull_request", number: 1896, url: "https://git.example/armada/pull/1896" } as const;
const TITLE = "Wait on the channel, not the sleep";
const entry = (id: number, minutes: number, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id,
  at: ago(minutes),
  kind,
  subject: PULL,
  title: TITLE,
  tone,
  says,
  manifest_id: "armada",
  ...rest,
});

// Ids above every entry `fleet-timeline` numbers.
const SESSION_PULL: TimelineEvent[] = [
  entry(1001, 9, "pr.opened", "idle", "Opened", { detail: "fix/flaky-store" }),
  entry(1002, 6, "pr.checks", "run", "Checks running", { live: true }),
  entry(1003, 3, "pr.checks", "ok", "Check store passed", { detail: "store" }),
];

export const s242CockpitTimelineFirst: Scenario = {
  ...base,
  name: "cockpit-timeline-first",
  says: "The Cockpit on its timeline: a Job and a Session each carry a pull request as marks on their own track, and no pull request has a row",
  draft: { ...base.draft, events: [...(base.draft?.events ?? []), ...SESSION_PULL] },
};
