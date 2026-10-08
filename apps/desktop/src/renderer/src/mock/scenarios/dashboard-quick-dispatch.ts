// The Dashboard with nothing waiting on the owner: a Job at work, a Job done, no Session and no
// merge line asking anything. Command Central is the quick dispatch box. The walk
// `dashboard-quick-dispatch` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import { completedSuccess } from "@armada/jobs/fixtures/build/index";
import { featureRunning } from "@armada/jobs/fake";

import { asRow, holding } from "../holding";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { Scenario } from "../moment";

const working = asRow(featureRunning(), 71, "dash-running", "Pin the store clock");
const done = asRow(completedSuccess(), 72, "dash-done", "Fold the two notification routes into one");

/** The recording's Job was cleared after it ran; this one is on the Board's Running section instead. */
function uncleared(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, owner_manifest_id: repository().manifest!.id };
  return { ...fixture, job };
}

const base = holding("dashboard-quick-dispatch", "The Dashboard with nothing needing the owner: the quick dispatch box", [uncleared(working), { ...done, job: { ...done.job, owner_manifest_id: repository().manifest!.id } }]);

// One repository picked, so the composer opens straight on its field rather than asking which.
export const s205DashboardQuickDispatch: Scenario = { ...base, state: { ...base.state, repository: repository().root } };
