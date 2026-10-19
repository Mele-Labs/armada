// `restyle-review`'s whole ship with Studios kept in two repositories, plus a pull request that merged three days ago.
// The timeline draws ages and a press turns them to clocks, where only the old event carries a day; the Studios list
// is touched on days before today. The walk `time-format-sweep` plays it.

import type { TimelineEvent } from "@armada/protocol";

import type { Scenario } from "../moment";
import { restyleReview } from "./restyle-review";

const DAY = 24 * 60 * 60_000;
const MERGED: TimelineEvent = {
  id: 2001,
  at: new Date(Date.now() - 3 * DAY).toISOString(),
  kind: "pr.merged",
  subject: { kind: "pull_request", number: 1850, url: "https://git.example/armada/pull/1850" },
  title: "Pin the clock in the store tests",
  tone: "ok",
  says: "Merged",
  manifest_id: "armada",
  over: true,
};

export const timeFormatSweep: Scenario = {
  ...restyleReview,
  name: "time-format-sweep",
  says: "The Cockpit's timeline with an event from three days ago, and Studios last touched on earlier days",
  draft: { ...restyleReview.draft, events: [...(restyleReview.draft?.events ?? []), MERGED] },
};
