import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { MergeLinePlace, type PlaceRow } from "./MergeLine";

/** The line one pull request is in, drawn under it elsewhere: named, in place order, this one current and the rest set back. */
const meta = {
  title: "Compositions/Merge line/Place",
  component: MergeLinePlace,
  parameters: { layout: "padded" },
  args: { onOpenPullRequest: fn(), rows: [] },
} satisfies Meta<typeof MergeLinePlace>;

export default meta;
type Story = StoryObj<typeof meta>;

const PULL = "https://git.example/armada/pull/";

const QUEUE: PlaceRow[] = [
  { place: 1, number: 1843, title: "Retry the forge read on a reset", url: `${PULL}1843`, mark: { line: "queue", state: "awaiting_checks" } },
  { place: 2, number: 1846, title: "Cap the log reader's buffer", url: `${PULL}1846`, mark: { line: "queue", state: "queued" } },
  { place: 3, number: 1850, title: "Record each migration once", url: `${PULL}1850`, mark: { line: "queue", state: "queued" }, current: true },
  { place: 4, number: 1852, title: "Rename the store test fixtures", url: `${PULL}1852`, mark: { line: "queue", state: "queued" } },
];

/** In the merge queue: the eyebrow names the line, the rows run in order, this pull request is current, and a row opens its own address. */
export const InTheQueue: Story = {
  args: { rows: QUEUE },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("region", { name: "Merge line" })).toBeInTheDocument();
    const rows = canvas.getAllByRole("listitem");
    await expect(rows.map((one) => one.getAttribute("aria-current"))).toEqual([null, null, "true", null]);
    await expect(canvas.queryByText(/of 4|ahead/)).toBeNull();
    await userEvent.click(canvas.getByRole("link", { name: /Cap the log reader's buffer/ }));
    await expect(args.onOpenPullRequest).toHaveBeenCalledWith(`${PULL}1846`);
  },
};
