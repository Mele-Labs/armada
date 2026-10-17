// A Session's pull request in the repository's merge queue: two pull requests ahead of #1850, which
// is queued, and one behind. #1849, the Session's other, is in no queue. The walk
// `session-pr-merge-line` plays it.

import type { MergeLine } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const PULL = "https://example.com/pull/";

function line(): MergeLine {
  const pull = (number: number, title: string, branch: string, position: number, state: string) => ({
    number,
    title,
    branch,
    url: `${PULL}${number}`,
    ci: "passed",
    queue: { state, position },
  });
  return {
    root: repository().root,
    line: [],
    off: [],
    landed: [],
    sent_back: [],
    hub: {
      pull_requests: [
        pull(1852, "Rename the store test fixtures", "chore/store-fixture-names", 4, "queued"),
        pull(1850, "Record each migration once", "spike/store-migrations-record", 3, "queued"),
        pull(1843, "Retry the forge read on a reset", "fix/forge-read-retry", 1, "awaiting_checks"),
        pull(1846, "Cap the log reader's buffer", "fix/cap-log-reader", 2, "queued"),
        { number: 1849, title: "Apply named migrations in order", branch: "spike/store-migrations", url: `${PULL}1849` },
      ],
    },
  };
}

export const s205SessionMergeLine: Scenario = {
  ...s200Sessions,
  name: "session-merge-line",
  says: "A Session's pull request in the merge queue, with others ahead and behind it",
  state: { ...s200Sessions.state, mergeLines: { lines: [line()] } },
};
