import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { MergeLine, type MergeLineEntry } from "./MergeLine";

/**
 * The merge line as an Overview panel: `armada land --status` drawn, from the
 * run of 2 Oct 2026. One story per place a row can be: in line alone, in a
 * batch, recently landed and sent back with each outcome, and a line nothing
 * has landed in.
 */
const meta = {
  title: "Compositions/Merge line",
  component: MergeLine,
  parameters: { layout: "padded" },
  args: { open: true, onOpenChange: fn(), onOpenPullRequest: fn() },
} satisfies Meta<typeof MergeLine>;

export default meta;
type Story = StoryObj<typeof meta>;

const PULL = "https://git.example/armada/pull/";
const TURN = "docs/wire-lock-signed";

const LINE: MergeLineEntry[] = [
  { place: 1, branch: "fleet/helm-kills-processes", state: "waiting" },
  { place: 2, branch: "docs/wire-lock-signed", state: "gating", batch: TURN, doing: "reading verify-foundations against main" },
  { place: 3, branch: "worktree-agent-aef3c24792026e2c3", state: "gating", batch: TURN, doing: "reading verify-foundations against main" },
  { place: 4, branch: "worktree-agent-a0087811ec86c6d80", state: "gating", batch: TURN, doing: "merging main (c527f60e09) into fleet/gate-policy-every-run" },
  { place: 5, branch: "fleet/gate-policy-every-run", state: "gating", batch: TURN, doing: "merging main (c527f60e09) into fleet/gate-policy-every-run" },
  { place: 6, branch: "fleet/read-in-cluster-membership", pr: { number: 1770, url: `${PULL}1770` }, state: "waiting" },
];

const LANDED: MergeLineEntry[] = [
  { branch: "studio/read-in-lands-in-a-zone", pr: { number: 1772, url: `${PULL}1772` }, state: "landed", merge: "007088d7ea" },
  { branch: "bridge/land-board-reads-plainly", state: "landed", merge: "29064cc27a" },
];

const SENT_BACK: MergeLineEntry[] = [
  { branch: "fleet/pulse-log-rows", pr: { number: 1768, url: `${PULL}1768` }, state: "red", failed: ["desktop_test", "screens_test"] },
  { branch: "bridge/overview-strip-width", state: "conflict", conflicts: ["apps/desktop/src/renderer/src/App.tsx"] },
  { branch: "fleet/drone-quiet-limit", state: "stopped" },
];

/** Six in line, the four between the waiting ones gating as one batch. */
export const InLine: Story = {
  name: "In line, with a batch",
  args: { line: LINE },
  // The batch is one group holding its four members, and the two waiting
  // either side of it are not in it.
  play: async ({ canvas }) => {
    const batch = canvas.getByRole("list", { name: "Batch" });
    const members = within(batch).getAllByRole("listitem");
    await expect(members.map((one) => one.getAttribute("aria-label"))).toEqual([
      "docs/wire-lock-signed, gating",
      "worktree-agent-aef3c24792026e2c3, gating",
      "worktree-agent-a0087811ec86c6d80, gating",
      "fleet/gate-policy-every-run, gating",
    ]);
    await expect(within(batch).queryByText("fleet/helm-kills-processes")).toBeNull();
    await expect(within(batch).queryByText("fleet/read-in-cluster-membership")).toBeNull();
  },
};

/** The line, then what landed with its merge, then what was sent back: red with its Checks, a conflict, a stop. */
export const Off: Story = {
  name: "Off the line",
  args: { line: LINE, landed: LANDED, sentBack: SENT_BACK },
  play: async ({ canvas }) => {
    const landed = canvas.getByRole("list", { name: "Recently landed" });
    await expect(within(landed).getAllByRole("listitem").map((one) => one.getAttribute("aria-label"))).toEqual([
      "studio/read-in-lands-in-a-zone, landed",
      "bridge/land-board-reads-plainly, landed",
    ]);
    const sent = canvas.getByRole("list", { name: "Sent back" });
    await expect(within(sent).queryByText("bridge/land-board-reads-plainly")).toBeNull();
    await expect(within(sent).getAllByRole("listitem")).toHaveLength(3);
  },
};

/** One of several, named by its repository beside the heading. */
export const Named: Story = {
  name: "Named by its repository",
  args: { name: "armada", line: LINE.slice(0, 1), landed: LANDED },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("region", { name: "Merge line, armada" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Collapse Merge line, armada" })).toBeVisible();
  },
};

/** The pulse, on. Under reduced motion the gating marks hold still and keep their hue. */
export const Pulsing: Story = {
  name: "Gating, with motion",
  args: { line: LINE.slice(1, 5) },
  parameters: { motion: "on" },
};

/** Nothing in line, and only what landed. No sentence says the line is empty, and Sent back draws nothing. */
export const NothingInLine: Story = {
  name: "Nothing in line",
  args: { line: [], landed: LANDED.slice(0, 1) },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("list", { name: "In line" })).toBeNull();
    await expect(canvas.getByRole("list", { name: "Recently landed" })).toBeVisible();
    await expect(canvas.queryByText("Sent back")).toBeNull();
  },
};

/** Nothing in line, landed or sent back: the picture, and not one word under it. */
export const NeverLanded: Story = {
  name: "Nothing ever landed",
  args: { line: [] },
  play: async ({ canvas, canvasElement }) => {
    await expect(canvas.getByRole("img", { name: "Empty" })).toBeVisible();
    await expect(canvasElement.querySelector(".armada-merge-line__empty")?.textContent).toBe("");
  },
};

/** Folded to its head, and no count beside it. */
export const Folded: Story = {
  name: "Folded",
  args: { line: LINE, open: false },
  play: async ({ canvas, args, userEvent }) => {
    await expect(canvas.queryByRole("list")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Expand Merge line" }));
    await expect(args.onOpenChange).toHaveBeenCalledWith(true);
  },
};
