import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { PullRequestActs } from "./PullRequestActs";

/** What a person does to a pull request from Bridge: ready it, merge it, ask for auto-merge, or have it reviewed. */
const meta: Meta<typeof PullRequestActs> = {
  title: "Compositions/Pull request acts",
  component: PullRequestActs,
  args: { onAct: fn(), auto: false },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--bg-raised)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof PullRequestActs>;

/** A draft is readied before anything else. */
export const Draft: Story = {
  args: { state: "draft", checks: "pending" },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Ready for review" }));
    await expect(args.onAct).toHaveBeenCalledWith("ready");
    await expect(canvas.queryByRole("button", { name: "Merge" })).toBeNull();
  },
};

/** Checks still running: auto-merge. */
export const ChecksRunning: Story = {
  args: { state: "open", checks: "pending" },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Enable auto-merge" }));
    await expect(args.onAct).toHaveBeenCalledWith("auto_merge");
  },
};

/** Auto-merge asked for: the fact, and no second ask. */
export const AutoMergeOn: Story = {
  args: { state: "open", checks: "pending", auto: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Auto-merge on" })).toBeDisabled();
  },
};

/** In the forge's merge queue: the fact, and nothing asks for it again. */
export const InMergeQueue: Story = {
  args: { state: "open", checks: "passed", auto: true, queued: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "In merge queue" })).toBeDisabled();
    await expect(canvas.queryByRole("button", { name: "Merge" })).toBeNull();
  },
};

/** Every Check passed: merge. */
export const Passing: Story = {
  args: { state: "open", checks: "passed" },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Merge" }));
    await expect(args.onAct).toHaveBeenCalledWith("merge");
  },
};

/** Red: nothing merges it, and Review is still there. */
export const Failing: Story = {
  args: { state: "open", checks: "failed" },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Merge" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Review" })).toBeInTheDocument();
  },
};

/** Landed: no acts. */
export const Merged: Story = {
  args: { state: "merged", checks: "passed" },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("group")).toBeNull();
  },
};
