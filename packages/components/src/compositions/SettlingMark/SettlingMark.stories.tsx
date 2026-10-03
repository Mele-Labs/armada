import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor } from "storybook/test";

import { SettlingMark } from "./SettlingMark";

/** A field the proposer has not written yet. One state: it is settling, or it is not drawn. */
const meta: Meta<typeof SettlingMark> = {
  title: "Compositions/Settling mark",
  component: SettlingMark,
};
export default meta;

type Story = StoryObj<typeof SettlingMark>;

/** **A bare mark names itself on hover**, naming the field it stands in for. */
export const Workflow: Story = {
  args: { field: "Workflow" },
  play: async ({ canvas, userEvent }) => {
    const mark = canvas.getByRole("img", { name: "Workflow, still being settled" });
    await userEvent.hover(mark);
    await waitFor(() => expect(canvas.getByText("Workflow, still being settled")).toBeVisible());
  },
};
