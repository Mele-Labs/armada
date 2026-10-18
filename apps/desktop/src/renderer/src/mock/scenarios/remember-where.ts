// Dispatch over two repositories with no Manifest, whose runs Fleet saw: Remember asks where they are kept at a repository's first, and not after.
// Over a connected Fleet with `my-app` picked. The walk `remember-where` plays it.

import type { RepositorySummary } from "@armada/protocol";

import { API, MY_APP, rememberWhereStore } from "../remember-where";
import { connected } from "../moment";
import type { Scenario } from "../moment";

const repository = (root: string): RepositorySummary => ({ root, records_root: `/Users/user/Library/Application Support/Armada/records/${root.split("/").pop()}` });

export const s302RememberWhere: Scenario = {
  name: "remember-where",
  says: "Two repositories with no Manifest, whose runs Fleet saw: Remember asks where at a repository's first",
  state: { ...connected([], [repository(MY_APP), repository(API)]), repository: MY_APP },
  draft: { sessions: () => rememberWhereStore() },
};
