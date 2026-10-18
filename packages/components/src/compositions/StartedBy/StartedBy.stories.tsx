import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { StartedBy } from "./StartedBy";

/** The Job whose agent started this one, a row that opens it. */
const meta: Meta<typeof StartedBy> = {
  title: "Compositions/Started by",
  component: StartedBy,
};
export default meta;

type Story = StoryObj<typeof StartedBy>;

export const Row: Story = {
  args: { title: "Debug Job 14", onOpen: fn() },
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Started by Debug Job 14" }));
    await expect(args.onOpen).toHaveBeenCalled();
  },
};
