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

/** Job 2 as Fleet served it at its gate: no title read, and nobody had commented. */
export const AsServed: Story = {
  args: { ...JOB_2, comments: 0 },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    // No title was served, so the name is the number alone and nothing stands in.
    await expect(canvas.getByRole("link", { name: "Pull request #1750" })).toBeVisible();
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
