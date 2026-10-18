// One side panel for a pull request, an issue, a bay and a Check, over the flight deck's ship with the Reviews seed. Marta's
// pull request #1899 is in armada's merge queue row and on his review list and nobody here owns it; #1896 is a Job's, its Checks
// failed. A Job's thread names issue #2214, and #1892's description closes it. The walk `shared-side-panel` plays it; the
// reads the panel draws from are `../shared-panel-reads.ts`.

import type { MergeLines } from "@armada/protocol";

import type { Scenario } from "../moment";
import { sharedPanelReads } from "../shared-panel-reads";
import { s262PrReview as base } from "./pr-review";

const FORGE = "https://forge.example/mele-labs/armada/pull/";

/** The same lines with marta's pull request open in armada's, checks passed. */
const withMarta = (lines: MergeLines): MergeLines => ({
  lines: lines.lines.map((line) =>
    line.hub === undefined || line.hub.pull_requests?.some((one) => one.number === 1892) !== true
      ? line
      : {
          ...line,
          hub: {
            ...line.hub,
            pull_requests: [...(line.hub.pull_requests ?? []), { number: 1899, title: "Cache the plan windows for a minute", branch: "marta/plan-window-cache", url: `${FORGE}1899`, ci: "passed", author: "marta", opened_at: new Date(Date.now() - 35 * 60_000).toISOString() }],
          },
        },
  ),
});

const LINES = withMarta(base.state.mergeLines ?? { lines: [] });

export const s263SharedSidePanel: Scenario = {
  ...base,
  name: "shared-side-panel",
  says: "A pull request, an issue, a bay and a Check, each opened in the one side panel from two places",
  state: { ...base.state, mergeLines: LINES },
  later: (base.later ?? []).map((one) => (one.mergeLines === undefined || one.mergeLines === null ? one : { ...one, mergeLines: withMarta(one.mergeLines) })),
  draft: {
    ...base.draft,
    // A Job's thread names an issue.
    sessions: (board, more) => {
      const store = base.draft!.sessions!(board, more);
      store.edit("q1", (one) => ({
        ...one,
        rows: [...one.rows, { id: "q1-b", at: "14:10:10", kind: "message", from: { kind: "agent" }, text: "It has the shape of #2214: a list rebuilt from recent writes." }],
      }));
      return store;
    },
    sharedPanel: () => sharedPanelReads(LINES),
  },
};
