// The Dashboard on All repositories with nothing waiting on the owner, and three set-up repositories
// served: armada with a Job at work and one done, storefront and billing with none. The dispatch bar
// has the cursor, and no repository is picked, so the panel it grows into reads one from the words.
// The walk `dispatch-grows-from-the-bar` plays it.

import type { RepositorySummary } from "@armada/protocol";
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

/** A repository served beside armada, set up, with nothing on the Board. */
function elsewhere(name: string): RepositorySummary {
  return {
    root: `/Users/user/code/${name}`,
    records_root: `/records/${name}`,
    manifest: { ...repository().manifest!, id: name, repository: name, path: `${name}/armada.yml` },
  };
}

const SERVED = [repository(), elsewhere("storefront"), elsewhere("billing")];

const base = holding(
  "dispatch-grows-from-the-bar",
  "The Dashboard on All repositories, nothing needing the owner: the dispatch bar over three set-up repositories",
  [uncleared(working), { ...done, job: { ...done.job, owner_manifest_id: repository().manifest!.id } }],
);

// The fixtures' own repositories are replaced by the three, so the inline list names exactly these.
export const s209DispatchGrowsFromTheBar: Scenario = {
  ...base,
  state: { ...base.state, holds: { ...base.state.holds, repositories: SERVED, manifests: SERVED.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest])) } },
};
