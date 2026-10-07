import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";

import { ForkedFrom } from "./ForkedFrom";

/** The old conversation a fork copied, closed until pressed. */
const meta: Meta<typeof ForkedFrom> = {
  title: "Compositions/Forked from",
  component: ForkedFrom,
};
export default meta;

type Story = StoryObj<typeof ForkedFrom>;

export const Closed: Story = {
  args: { title: "Why the reader drops lines", children: <p>It splits on newlines.</p> },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByText("Forked from Why the reader drops lines"));
    await expect(canvas.getByText("It splits on newlines.")).toBeVisible();
  },
};
