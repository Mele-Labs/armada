import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { PullRequestChip } from "./PullRequestChip";

/** A pull request as a Job's Progress lists it, and framed, as a chip among cards. */
const meta: Meta<typeof PullRequestChip> = {
  title: "Compositions/Pull request chip",
  component: PullRequestChip,
  args: { number: 2814, title: "Compare coupon expiry in UTC", state: "open", checks: { state: "passed", said: "Checks passed" }, onOpen: fn() },
  decorators: [(Story) => <div style={{ width: 360 }}><Story /></div>],
};
export default meta;

type Story = StoryObj<typeof PullRequestChip>;

export const Open: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "Open" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Checks passed" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Pull request #2814 Compare coupon expiry in UTC" }));
    await expect(args.onOpen).toHaveBeenCalledOnce();
  },
};

export const ChecksRunning: Story = { args: { checks: { state: "running", said: "Checks running" } } };

export const ChecksFailed: Story = { args: { checks: { state: "failed", said: "Checks failed: unit" } } };

export const Merged: Story = { args: { state: "merged" } };

export const Draft: Story = { args: { state: "draft", checks: undefined } };

export const FramedOpen: Story = { args: { framed: true } };

export const FramedFailed: Story = { args: { framed: true, checks: { state: "failed", said: "Checks failed: unit" } } };

export const FramedMerged: Story = { args: { framed: true, state: "merged" } };

export const FramedClosed: Story = { args: { framed: true, state: "closed", checks: undefined } };
