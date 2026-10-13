import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import type { PlaceRow } from "../MergeLine/MergeLine";
import { PullRequestPanel } from "./PullRequestPanel";

const ADDRESS = "https://github.com/mele-labs/armada/pull/";

/** A pull request as its panel draws it, with its status, queue and failing checks as plates off the leading edge. */
const meta = {
  title: "Compositions/Pull request panel",
  component: PullRequestPanel,
  parameters: { layout: "fullscreen" },
  args: {
    number: 2209,
    title: "Open pull requests in Armada",
    branch: "claude/pr-panel-redesign",
    address: `${ADDRESS}2209`,
    repository: "mele-labs/armada",
    state: "open",
    checks: { state: "passed" },
    auto: false,
    onAct: fn(),
    onOpenLink: fn(),
    onClose: fn(),
  },
} satisfies Meta<typeof PullRequestPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

const row = (place: number, number: number, title: string, state: "queued" | "awaiting_checks" | "mergeable"): PlaceRow => ({
  place,
  number,
  title,
  url: `${ADDRESS}${number}`,
  mark: { line: "queue", state },
});

const QUEUE: PlaceRow[] = [
  row(1, 2201, "Commander repair onto the Job's branch", "mergeable"),
  row(2, 2204, "Keep the dispatch bar under Helm", "awaiting_checks"),
  { ...row(3, 2209, "Open pull requests in Armada", "queued"), current: true },
  row(4, 2211, "Settings: per-repo merge method", "queued"),
  row(5, 2212, "Tidy the verdict sheet", "queued"),
  row(6, 2215, "Wire Cockpit holds", "queued"),
  row(7, 2216, "Spike 031 notes", "queued"),
];

/** In the merge queue, third: the place figure on the status plate, the queue plate with this pull request current. */
export const InTheMergeQueue: Story = {
  args: { queued: true, place: QUEUE },
};

/** Every check passed: the Merge act and the status plate alone. */
export const ReadyToMerge: Story = {};

/** The description its author wrote, drawn as Markdown at the end of the body. */
export const WithADescription: Story = {
  args: {
    body: "Pull request links open the same way everywhere, behind a setting: in Armada or in the browser.\n\n- `links.openPullRequestsIn` picks which\n- Open in browser stays on the panel",
  },
};

/** The forge's checks failed: the failing plate names them, and Notify Session tells the Session. */
export const ChecksFailed: Story = {
  args: { checks: { state: "failed", failing: ["test · bridge", "verify-foundations"] }, onSendBack: fn() },
};
