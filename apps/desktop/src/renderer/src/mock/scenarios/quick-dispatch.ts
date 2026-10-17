// The Cockpit on All repositories with nothing waiting on the owner, four set-up repositories served
// (armada with a Job at work and one done, storefront, billing and docs-site with none) and Sessions
// served. No repository is picked, so the dispatch panel reads one from the words and draws the four as
// a row of chips. The walks `quick-dispatch-deck-button` and `dispatched-strip-opens-it` play it, and so
// does `dispatch-panel.test.tsx`.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { completedSuccess } from "@armada/screens/src/fixtures/build/index";
import { featureRunning } from "@armada/screens/src/fixtures/jobs/feature-running";

import { asRow, holding } from "../holding";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const working = asRow(featureRunning(), 71, "dash-running", "Pin the Bridge store clock");
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

const SERVED = [repository(), elsewhere("storefront"), elsewhere("billing"), elsewhere("docs-site")];

const base = holding(
  "quick-dispatch",
  "The Cockpit on All repositories, nothing needing the owner: the dispatch panel in one press, over four set-up repositories",
  [uncleared(working), { ...done, job: { ...done.job, owner_manifest_id: repository().manifest!.id } }],
);

// The fixtures' own repositories are replaced by the four, so the row of chips names exactly these.
const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };

const owners = new Map(SERVED.flatMap((one) => (one.manifest === undefined ? [] : [[one.root, one.manifest.id] as const])));

export const s210QuickDispatch: Scenario = {
  ...base,
  state: { ...base.state, holds: { ...base.state.holds, repositories: SERVED, manifests: SERVED.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest])) } },
  // A Session starts in the repository it is given and is listed under it, as Fleet lists one by the Manifest it started in.
  draft: { ...base.draft, sessions: (board) => sessionsStore([none, none], none, board, [], [], [], owners) },
};
