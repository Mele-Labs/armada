// A repository with workspaces whose runs Fleet saw pass, all kept on this Mac: its row in the repository picker opens them as a layer of workspaces.
// Over a connected Fleet with `my-app` picked. The walk `runs-at-scale` plays it.

import type { RepositorySummary } from "@armada/protocol";

import { MY_APP, RUNS } from "../runs-at-scale";
import { sessionsStore } from "../sessions/script";
import { connected } from "../moment";
import type { Scenario } from "../moment";

const repository: RepositorySummary = { root: MY_APP, records_root: "/Users/user/Library/Application Support/Armada/records/my-app" };

export const s303RunsAtScale: Scenario = {
  name: "runs-at-scale",
  says: "A repository with workspaces, its Armada configuration kept on this Mac: its row in the picker opens it",
  state: { ...connected([], [repository]), repository: MY_APP },
  draft: { sessions: () => sessionsStore([], [], new Map(), []), runFacts: RUNS },
};
