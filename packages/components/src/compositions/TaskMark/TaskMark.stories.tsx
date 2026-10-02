import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor } from "storybook/test";
import { TaskMark } from "./TaskMark";

/** One story per task state — the whole of the mark's vocabulary. */
const meta: Meta<typeof TaskMark> = {
  title: "Compositions/Task mark",
  component: TaskMark,
};
export default meta;

type Story = StoryObj<typeof TaskMark>;

export const Open: Story = { args: { state: "open" } };
export const Working: Story = { args: { state: "working" } };

/**
 * **The one state whose hover says more than its name.** The name is still the
 * registry's word; the tooltip adds what the task is waiting on.
 */
export const HandedIn: Story = {
  args: { state: "handed_in" },
  play: async ({ canvas, userEvent }) => {
    const mark = canvas.getByRole("img", { name: "Handed in" });
    await userEvent.hover(mark);
    await waitFor(() => expect(canvas.getByText("Handed in, waiting for its Checks")).toBeVisible());
  },
};

/**
 * **A bare glyph names itself on hover.** Nothing beside the mark prints the
 * state, so the tooltip is the only place a pointer reads it, in the
 * registry's own word.
 */
export const Done: Story = {
  args: { state: "done" },
  play: async ({ canvas, userEvent }) => {
    const mark = canvas.getByRole("img", { name: "Done" });
    await expect(canvas.getByText("Done")).not.toBeVisible();
    await userEvent.hover(mark);
    await waitFor(() => expect(canvas.getByText("Done")).toBeVisible());
  },
};
export const Failed: Story = { args: { state: "failed" } };
export const Dropped: Story = { args: { state: "dropped" } };
