// Armada reads and raises calls for the pull requests its person opened, and for no one else's. A teammate has two open: #1901 is in
// the merge queue behind the person's own #1890, and #1902 is not in it and has failing checks. Fleet marks both `mine: false`. The
// queue draws #1901 as a dot with its place and no press, #1902 is not drawn anywhere, and the person's own failing #1896 raises a call.
// The walk `onlyMyPrs` plays it.

import type { HubPullRequest } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const behindMine: HubPullRequest = {
  number: 1901,
  title: "Rework the sidebar width",
  branch: "mia/sidebar-width",
  url: `${FORGE}1901`,
  author: "mia",
  ci: "failed",
  queue: { state: "queued", position: 2 },
  opened_at: ago(55),
  mine: false,
};
const outsideTheQueue: HubPullRequest = {
  number: 1902,
  title: "Retry the upload",
  branch: "mia/retry-upload",
  url: `${FORGE}1902`,
  author: "mia",
  ci: "failed",
  opened_at: ago(20),
  mine: false,
};

export const s262OnlyMyPrs: Scenario = {
  ...ship,
  name: "only-my-prs",
  says: "The flight deck with a teammate's pull request in the merge queue and another outside it, over the person's own failing one",
  state: {
    ...ship.state,
    mergeLines: {
      lines: ship.state.mergeLines!.lines.map((line, at) =>
        at !== 0 ? line : { ...line, hub: { ...line.hub, pull_requests: [...(line.hub?.pull_requests ?? []), behindMine, outsideTheQueue] } },
      ),
    },
  },
};
