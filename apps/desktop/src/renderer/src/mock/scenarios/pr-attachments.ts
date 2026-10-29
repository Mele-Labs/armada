// A pull request whose description has a Demo section: a bare attachment address on a line of its own, and a picture. Every read
// is `shared-side-panel`'s. The walk `pr-attachments` plays it.

import type { MergeLines } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s263SharedSidePanel as base } from "./shared-side-panel";

const FORGE = "https://forge.example/mele-labs/armada/pull/";
const ADDED = 1895;

const withDemo = (lines: MergeLines): MergeLines => ({
  ...lines,
  lines: lines.lines.map((line) =>
    line.hub === undefined || line.hub.pull_requests?.some((one) => one.number === 1899) !== true
      ? line
      : {
          ...line,
          hub: {
            ...line.hub,
            pull_requests: [...(line.hub.pull_requests ?? []), { number: ADDED, title: "Show the pause marker in the list", branch: "bridge/pause-marker-row", url: `${FORGE}${ADDED}`, ci: "passed", author: "claude", opened_at: new Date(Date.now() - 20 * 60_000).toISOString() }],
          },
        },
  ),
});

export const s265PrAttachments: Scenario = {
  ...base,
  name: "pr-attachments",
  says: "A pull request's description plays its demo video inline and draws its picture",
  state: { ...base.state, mergeLines: base.state.mergeLines === null || base.state.mergeLines === undefined ? null : withDemo(base.state.mergeLines) },
  later: (base.later ?? []).map((one) => (one.mergeLines === undefined || one.mergeLines === null ? one : { ...one, mergeLines: withDemo(one.mergeLines) })),
};
