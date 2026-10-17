// Two repositories on All, each with a backlog: storefront's issues are on the forge
// and read #412; ops keeps them in Jira and reads ABC-123 and OPS-7. The walk `issue-backlogs` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet } from "../sessions-fleet";

const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};
const OPS: RepositorySummary = {
  root: "/Users/user/code/ops",
  records_root: "/records/ops",
  manifest: { ...repository().manifest!, id: "ops", repository: "ops", path: "ops/armada.yml" },
};

const board = onBoard({ repositories: [STOREFRONT, OPS] });

/** Served as the Sessions fleet serves a scenario, so Helm's box is the Sessions box and `#` reads each repository's issues. */
function build(): Scenario {
  const served = new FakeSessionsFleet([], {}).scenario(board);
  return {
    ...served,
    name: "issue-backlogs",
    says: "Two repositories on All: the composer's open issues from the forge, and from Jira",
    behaves: (handle) => new FakeSessionsFleet([], {}).scenario(board).behaves!(handle),
  };
}

export const s246IssueBacklogs: Scenario = build();
