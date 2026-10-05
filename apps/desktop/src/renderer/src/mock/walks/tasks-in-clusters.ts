// The running canvas with each group a dashed Cluster holding its tasks
// (prototype, 5 Oct 2026), beside `tasksOffTheGroups`' chains: the same plan,
// to compare. A task opens Plan's own panel; the cluster's head opens the group.

import { button, card, region, role, text, walk } from "../walk";

const RUN = region("This Job's run");

export const tasksInClusters = walk(
  "proto/feature-running",
  [
    { look: RUN, say: "Each group is a Cluster side by side under Implement, its tasks inside" },
    { look: role("button", /^Group 3, running/), say: "The group at work has the live frame" },
    { look: role("button", /^Group 1, passed/), say: "and a passed one recedes" },
    { look: card("Checks"), say: "The Clusters rejoin above the gate" },
    { press: role("button", /^Draw tasks under their group, /), say: "A task opens Plan's panel" },
    { look: text("Draw tasks under their group"), say: "T6, its own panel" },
    { press: button("Back to Overview"), say: "Back" },
    { press: role("button", /^Group 3, running/), say: "A Cluster's head opens the group" },
  ],
  { plan: "clusters" },
);
