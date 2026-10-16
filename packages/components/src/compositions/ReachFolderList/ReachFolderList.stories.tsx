import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ReachFolderList } from "./ReachFolderList";

const meta: Meta<typeof ReachFolderList> = {
  title: "Compositions/Reach folder list",
  component: ReachFolderList,
  args: { allowed: ["/Users/a/Documents/handoffs"], blocked: ["/Users/a/.ssh"], onRemove: fn() },
};
export default meta;

type Story = StoryObj<typeof ReachFolderList>;

/** A blocked folder and an allowed one. Remove hands back the folder. */
export const Listed: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "Always blocked" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Remove /Users/a/Documents/handoffs" }));
    await expect(args.onRemove).toHaveBeenCalledWith("/Users/a/Documents/handoffs");
  },
};

export const NoneYet: Story = { name: "None yet", args: { allowed: [], blocked: [] } };

export const NotConnected: Story = { name: "Not connected", args: { disabled: true } };
