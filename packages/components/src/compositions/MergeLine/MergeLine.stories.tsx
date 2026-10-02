import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { MergeLine, type MergeLineEntry } from "./MergeLine";

/**
 * The merge line as an Overview panel: `armada land --status` drawn, from the
 * run of 2 Oct 2026. One story per place a row can be: in line alone, in a
 * batch, and off the line with each outcome.
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

const OFF: MergeLineEntry[] = [
  { branch: "bridge/land-board-reads-plainly", state: "landed", merge: "29064cc27a" },
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

/** The line, and what just left it: landed with its merge, red with its Checks, a conflict, a stop. */
export const Off: Story = {
  name: "Off the line",
  args: { line: LINE, off: OFF },
};

/** The pulse, on. Under reduced motion the gating marks hold still and keep their hue. */
export const Pulsing: Story = {
  name: "Gating, with motion",
  args: { line: LINE.slice(1, 5) },
  parameters: { motion: "on" },
};

/** Nothing in line, and only what left it. No sentence says the line is empty. */
export const NothingInLine: Story = {
  name: "Nothing in line",
  args: { line: [], off: OFF.slice(0, 1) },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("list", { name: "In line" })).toBeNull();
    await expect(canvas.getByRole("list", { name: "Left the line" })).toBeVisible();
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
