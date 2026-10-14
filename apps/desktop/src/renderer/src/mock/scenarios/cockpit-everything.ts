// Every change to the Cockpit in one sitting: `cockpit-timeline-first`'s timeline, with a pull request on the
// track of the Job or Session that owns it, over `cockpit-calls-arrive`'s two calls, which arrive one after
// the other in the tray. No repository is picked, so Autopilot asks for one. The walk `cockpitEverything`
// plays it.

import type { Scenario } from "../moment";
import { cockpitCallsArrive as arriving } from "./cockpit-calls-arrive";
import { s242CockpitTimelineFirst as base } from "./cockpit-timeline-first";

/** The Job that asks in each of `cockpit-calls-arrive`'s later moments, the last Job it names. */
const asking = (moment: number) => {
  const jobs = arriving.later?.[moment];
  if (typeof jobs === "function" || jobs?.jobs === undefined) throw new Error(`cockpit-calls-arrive holds no Job at moment ${moment}`);
  return jobs.jobs[jobs.jobs.length - 1]!;
};
const plan = asking(1);
const check = asking(2);

export const cockpitEverything: Scenario = {
  ...base,
  name: "cockpit-everything",
  says: "The Cockpit on its timeline with pull requests on their tracks, Autopilot in the header band, and two calls that arrive in the tray one after the other",
  // The first moment is a beat with nothing in it: a call is only news once Bridge has settled on what it holds.
  later: [{}, { jobs: [...base.state.jobs, plan] }, { jobs: [...base.state.jobs, check] }],
  reads: { ...base.reads, [plan.id]: arriving.reads[plan.id]!, [check.id]: arriving.reads[check.id]! },
  draft: {
    ...base.draft,
    calls: { ...base.draft?.calls, ...arriving.draft?.calls },
    now: { ...base.draft?.now, ...arriving.draft?.now },
  },
};
