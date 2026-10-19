// `restyle-review`'s whole ship with Studios kept in two repositories. The timeline draws ages and a press turns them
// to clocks; the Studios list is touched on days before today, so each carries its day. The walk `time-format-sweep`
// plays it.

import type { Scenario } from "../moment";
import { restyleReview } from "./restyle-review";

export const timeFormatSweep: Scenario = {
  ...restyleReview,
  name: "time-format-sweep",
  says: "The Cockpit's timeline, and Studios last touched on earlier days",
};
