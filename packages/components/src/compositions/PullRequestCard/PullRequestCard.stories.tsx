import type { Meta, StoryObj } from "@storybook/react-vite";
import { GitMerge, GitPullRequestClosed } from "lucide-react";
import { expect, fn, userEvent, within } from "storybook/test";

import { PullRequestCard } from "./PullRequestCard";

const meta: Meta<typeof PullRequestCard> = {
  title: "Compositions/Pull request card",
  component: PullRequestCard,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-proposal-settings)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof PullRequestCard>;

const JOB_2 = {
  number: "#1750",
  address: "https://forge.invalid/armada/armada/pull/1750",
  branch: "armada/2-retire-guides-8-and-20-add-validation-that",
  checks: "21/21 Checks passed",
};

/** Open, with every fact read. The whole card is the press, and it opens through the host. */
export const Open: Story = {
  args: {
    ...JOB_2,
    title: "Retire guides 8 and 20, add validation that every guide's piece is drawn somewhere",
    comments: 2,
    onOpen: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const card = canvas.getByRole("link", { name: /^Pull request #1750, Retire guides/ });
    await expect(canvas.getByText("21/21 Checks passed")).toBeVisible();
    await expect(canvas.getByText("2 comments")).toBeVisible();
    await userEvent.click(card);
    await expect(args.onOpen).toHaveBeenCalledOnce();
  },
};

/** Job 2 as a 23.5 Fleet serves it at its gate: its title, and nobody had commented. */
export const AsServed: Story = {
  args: {
    ...JOB_2,
    title: "Retire guides 8 and 20, add validation that every guide's piece is drawn somewhere",
    comments: 0,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    // A served 0 is a count, so it is drawn.
    await expect(canvas.getByText("0 comments")).toBeVisible();
  },
};

/** A pull request from before 23.5 that no read has named since: no title, no count. */
export const BeforeTitles: Story = {
  args: JOB_2,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    // The name is the number alone, and nothing stands in for the count.
    await expect(canvas.getByRole("link", { name: "Pull request #1750" })).toBeVisible();
    await expect(canvas.queryByText(/comment/)).toBeNull();
  },
};

/** Merged: the badge the Job header and the Land board draw. */
export const Merged: Story = {
  args: {
    ...JOB_2,
    title: "Retire guides 8 and 20, add validation that every guide's piece is drawn somewhere",
    state: { status: "completed-success", icon: GitMerge, label: "Merged" },
    comments: 0,
  },
};

/** Closed without merging, in the neutral hue: nothing says why it closed. */
export const Closed: Story = {
  args: {
    ...JOB_2,
    state: { status: "not-started", icon: GitPullRequestClosed, label: "Closed without merging" },
  },
};

/** The forge's checks as a mark, its tooltip naming it, and auto-merge asked for beside it. */
export const ForgeMarks: Story = {
  args: { ...JOB_2, title: "Retire guides 8 and 20", forge: { checks: "running", autoMerge: true } },
  play: async ({ canvasElement }) => {
    const card = within(canvasElement);
    await expect(card.getByRole("img", { name: "Checks running" })).toBeVisible();
    await expect(card.getByRole("img", { name: "Auto-merge on" })).toBeVisible();
  },
};

export const ForgeChecksFailed: Story = {
  args: { ...JOB_2, forge: { checks: "failed", failing: ["ci / test", "ci / lint"] } },
  play: async ({ canvasElement }) => {
    const card = within(canvasElement);
    const mark = card.getByRole("img", { name: "Checks failed: ci / test, ci / lint" });
    const names = card.getByText("ci / test, ci / lint");
    await expect(mark).toBeVisible();
    // One group: the names sit on the mark's own row, not under it.
    expect(Math.abs(names.getBoundingClientRect().top - mark.getBoundingClientRect().top)).toBeLessThan(mark.getBoundingClientRect().height);
  },
};
