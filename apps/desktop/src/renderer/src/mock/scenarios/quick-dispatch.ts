// The Cockpit on All repositories with nothing waiting on the owner, four set-up repositories served
// (armada, storefront, billing and docs-site) and Sessions served. No repository is picked, so the
// dispatch panel reads one from the words and draws the four as a row of chips. The walks `quick-dispatch-deck-button` and `dispatched-strip-opens-it` play it, and so
// does `dispatch-panel.test.tsx`.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

/** A repository served beside armada, set up. */
function elsewhere(name: string): RepositorySummary {
  return {
    root: `/Users/user/code/${name}`,
    records_root: `/records/${name}`,
    manifest: { ...repository().manifest!, id: name, repository: name, path: `${name}/armada.yml` },
  };
}

const SERVED = [repository(), elsewhere("storefront"), elsewhere("billing"), elsewhere("docs-site")];

const base = holding("quick-dispatch", "The Cockpit on All repositories, nothing needing the owner: the dispatch panel in one press, over four set-up repositories");

const owners = new Map(SERVED.flatMap((one) => (one.manifest === undefined ? [] : [[one.root, one.manifest.id] as const])));

export const s210QuickDispatch: Scenario = {
  ...base,
  // The base repository is replaced by the four, so the row of chips names exactly these.
  state: { ...base.state, holds: { ...base.state.holds, repositories: SERVED, manifests: SERVED.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest])) } },
  // A Session starts in the repository it is given and is listed under it, as Fleet lists one by the Manifest it started in.
  draft: { ...base.draft, sessions: () => sessionsStore([], [], owners) },
};
