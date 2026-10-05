import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { MergeLine, type MergeLineEntry, type MergeLineNotice } from "./MergeLine";

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
  { place: 2, branch: "docs/wire-lock-signed", state: "preparing", batch: TURN, doing: "reading verify-foundations against main" },
  { place: 3, branch: "worktree-agent-aef3c24792026e2c3", state: "preparing", batch: TURN, doing: "reading verify-foundations against main" },
  { place: 4, branch: "worktree-agent-a0087811ec86c6d80", state: "preparing", batch: TURN, doing: "merging main (c527f60e09) into fleet/gate-policy-every-run" },
  { place: 5, branch: "fleet/gate-policy-every-run", state: "preparing", batch: TURN, doing: "merging main (c527f60e09) into fleet/gate-policy-every-run" },
  { place: 6, branch: "fleet/read-in-cluster-membership", pr: { number: 1770, url: `${PULL}1770` }, state: "waiting" },
];

const LANDED: MergeLineEntry[] = [
  { branch: "studio/read-in-lands-in-a-zone", pr: { number: 1772, url: `${PULL}1772` }, state: "landed", merge: "007088d7ea" },
  { branch: "bridge/land-board-reads-plainly", state: "landed", merge: "29064cc27a" },
];

const SENT_BACK: MergeLineEntry[] = [
  { branch: "fleet/pulse-log-rows", pr: { number: 1768, url: `${PULL}1768` }, state: "red", failed: ["desktop_test", "screens_test"] },
  {
    branch: "fleet/drone-quiet-window",
    state: "red",
    failed: ["desktop_test"],
    checks: [
      { name: "build", state: "passed" },
      { name: "desktop_test", state: "failed" },
      { name: "screens_test", state: "timed_out" },
    ],
  },
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
      "docs/wire-lock-signed, Preparing to land",
      "worktree-agent-aef3c24792026e2c3, Preparing to land",
      "worktree-agent-a0087811ec86c6d80, Preparing to land",
      "fleet/gate-policy-every-run, Preparing to land",
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
    await expect([...sent.querySelectorAll(":scope > li")].map((one) => one.getAttribute("aria-label"))).toEqual([
      "fleet/pulse-log-rows, Checks failed",
      "fleet/drone-quiet-window, Checks failed",
      "bridge/overview-strip-width, conflict",
      "fleet/drone-quiet-limit, stopped",
    ]);
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

/** A turn in its Checks: the boundary strip in place of the runner's words, one segment a Check. */
export const InItsChecks: Story = {
  name: "In its Checks",
  args: {
    line: [
      {
        place: 1,
        branch: "docs/wire-lock-signed",
        state: "gating",
        checks: [
          { name: "build", state: "passed" },
          { name: "screens_test", state: "running" },
          { name: "desktop_test", state: "waiting" },
        ],
      },
      { place: 2, branch: "fleet/gate-policy-every-run", state: "preparing", doing: "merging main (c527f60e09) into fleet/gate-policy-every-run" },
    ],
  },
  play: async ({ canvas }) => {
    const row = canvas.getByRole("listitem", { name: "docs/wire-lock-signed, Running Checks before landing" });
    await expect(within(row).getByRole("region", { name: "Checks at this boundary" })).toBeVisible();
    await expect(within(row).queryByText(/running screens_test/)).toBeNull();
    await expect(canvas.getByText("merging main (c527f60e09) into fleet/gate-policy-every-run")).toBeVisible();
  },
};

/** A detail that wraps keeps the place, mark and branch on its first line. */
export const Wrapping: Story = {
  name: "A detail that wraps",
  args: {
    line: [],
    sentBack: [
      {
        branch: "bridge/overview-strip-width",
        state: "conflict",
        conflicts: [
          "apps/desktop/src/renderer/src/App.tsx",
          "packages/screens/src/OverviewLists.tsx",
          "packages/screens/src/OverviewSummary.tsx",
          "packages/components/src/compositions/TheShell/TheShell.tsx",
          "packages/components/src/compositions/TheShell/TheShell.css",
        ],
      },
    ],
  },
  play: async ({ canvas, canvasElement }) => {
    const row = canvas.getByRole("listitem", { name: /overview-strip-width/ });
    const branch = within(row).getByText("bridge/overview-strip-width").getBoundingClientRect();
    const mark = within(row).getByRole("img").getBoundingClientRect();
    const detail = canvasElement.querySelector(".armada-merge-line__detail")!.getBoundingClientRect();
    await expect(detail.height).toBeGreaterThan(branch.height * 1.5);
    await expect(Math.abs(mark.top + mark.height / 2 - (branch.top + branch.height / 2))).toBeLessThan(2);
  },
};

/** A batch mid-turn: screens_test failed, its rerun failed, and desktop_test is running behind it. */
const FAILING: MergeLineEntry[] = [
  { place: 1, branch: "docs/wire-lock-signed", state: "gating", batch: TURN, doing: "reading verify-foundations against main" },
  {
    place: 2,
    branch: "worktree-agent-aef3c24792026e2c3",
    state: "gating",
    batch: TURN,
    checks: [
      { name: "build", state: "passed" },
      { name: "screens_test", state: "failed" },
      { name: "desktop_test", state: "running" },
      { name: "components_test", state: "waiting" },
    ],
  },
];

const AT = "worktree-agent-aef3c24792026e2c3";

const noticed = (notice: MergeLineNotice): Story => ({
  args: { line: FAILING, notice, onOpenCheck: fn() },
  play: async ({ canvas, args }) => {
    // One alert, whichever reading: a later one replaces it.
    await expect(canvas.getAllByRole("status")).toHaveLength(1);
    await canvas.getByRole("button", { name: "Open log" }).click();
    await expect(args.onOpenCheck).toHaveBeenCalledWith(AT, "screens_test");
  },
});

/** Before the split names the one at fault: a heads-up to the whole batch, once. */
export const FailedBatch: Story = {
  name: "A Check failed, batch not split",
  ...noticed({ kind: "batch", check: "screens_test", branch: AT }),
};

/** The split named the branch at fault while the turn runs on. */
export const FailedBranch: Story = {
  name: "A Check failed, branch named",
  ...noticed({ kind: "branch", check: "screens_test", branch: AT }),
};

/** The same Check is red on main: nobody is blamed, and the turn holds. */
export const FailedOnMain: Story = {
  name: "A Check red on main too",
  ...noticed({ kind: "main", check: "screens_test", branch: AT }),
};

/** The turn's verdict, in the slot the heads-up held. */
export const FailedSent: Story = {
  name: "A Check failed, branch sent back",
  args: {
    line: [{ place: 1, branch: "docs/wire-lock-signed", state: "preparing", doing: "reading verify-foundations against main" }],
    notice: { kind: "sent", check: "screens_test", branch: AT },
    sentBack: [{ branch: AT, state: "red", failed: ["screens_test"] }],
    onOpenCheck: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("status")).toHaveLength(1);
    await expect(canvas.getByRole("status")).toHaveTextContent(AT);
  },
};

/** Waiting branches in merge order, each with the reason it is not in the running turn. */
export const WaitingWhy: Story = {
  name: "Waiting, with the reason",
  args: {
    line: [
      { place: 1, branch: "canvas/gates-on-the-spine", state: "gating", batch: "t", checks: [{ name: "build", state: "running" }] },
      { place: 2, branch: "fix/overview-fallback-and-reads", state: "preparing", batch: "t", doing: "reading verify-foundations against main" },
      { place: 3, branch: "fleet/stranded-slot-rescue", state: "waiting", why: "kept" },
      { place: 4, branch: "fix/dispatch-did-not-answer", state: "waiting" },
      { place: 5, branch: "fix/pulse-log-clash", state: "waiting", why: "member" },
      { place: 6, branch: "fix/stale-base-read", state: "waiting", why: "main" },
      { place: 7, branch: "bridge/late-joiner", state: "waiting", why: "late" },
    ],
  },
  play: async ({ canvas }) => {
    const queued = canvas.getByRole("listitem", { name: "fix/dispatch-did-not-answer, waiting" });
    // Queued behind with no reason: a clock for the state and nothing else.
    await expect(within(queued).getAllByRole("img")).toHaveLength(1);
    const kept = canvas.getByRole("listitem", { name: "fleet/stranded-slot-rescue, waiting" });
    await expect(within(kept).getByRole("img", { name: /Kept its place/ })).toBeVisible();
  },
};
