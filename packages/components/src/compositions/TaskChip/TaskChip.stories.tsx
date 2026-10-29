import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { TaskChip } from "./TaskChip";

/** A task of his projects, as a Job's rail names one under its brief, and framed, as a chip among cards. */
const meta: Meta<typeof TaskChip> = {
  title: "Compositions/Task chip",
  component: TaskChip,
  args: { title: "Update the changelog", project: "Release notes", state: "held", onOpen: fn() },
  decorators: [(Story) => <div style={{ width: 360 }}><Story /></div>],
};
export default meta;

type Story = StoryObj<typeof TaskChip>;

export const Held: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "In a Job" })).toBeVisible();
    await expect(canvas.getByText("Release notes")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Task Update the changelog" }));
    await expect(args.onOpen).toHaveBeenCalledOnce();
  },
};

export const Open: Story = { args: { state: "open" } };

export const Asking: Story = { args: { state: "asking" } };

export const Done: Story = { args: { state: "done" } };

export const NoProject: Story = { args: { project: "No project" } };

export const LongTitle: Story = { args: { title: "Compare coupon expiry in UTC across every region the billing service reads its calendar from" } };

export const FramedOpen: Story = { args: { framed: true, state: "open" } };

export const FramedHeld: Story = { args: { framed: true } };

export const FramedAsking: Story = { args: { framed: true, state: "asking" } };

export const FramedDone: Story = { args: { framed: true, state: "done" } };
