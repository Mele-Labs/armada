// Dispatch over a repository with workspaces whose runs Fleet saw, most kept already: only the two new lines are drawn, and all of it is one
// press away. Over a connected Fleet with `my-app` picked. The walk `runs-at-scale` plays it.

import type { RepositorySummary } from "@armada/protocol";

import { MY_APP, runsAtScaleStore } from "../runs-at-scale";
import { connected } from "../moment";
import type { Scenario } from "../moment";

const repository: RepositorySummary = { root: MY_APP, records_root: "/Users/user/Library/Application Support/Armada/records/my-app" };

export const s303RunsAtScale: Scenario = {
  name: "runs-at-scale",
  says: "A repository with workspaces, most of how it runs kept already: Dispatch shows what is new",
  state: { ...connected([], [repository]), repository: MY_APP },
  draft: { sessions: () => runsAtScaleStore() },
};
