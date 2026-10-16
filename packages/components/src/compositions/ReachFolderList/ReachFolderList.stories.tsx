import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ReachFolderList } from "./ReachFolderList";

const meta: Meta<typeof ReachFolderList> = {
  title: "Compositions/Reach folder list",
  component: ReachFolderList,
  args: { allowed: ["/Users/user/Documents/handoffs"], blocked: ["/Users/user/.ssh"], onRemove: fn() },
};
export default meta;

type Story = StoryObj<typeof ReachFolderList>;

/** A blocked folder and an allowed one. Remove hands back the folder. */
export const Listed: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "Always blocked" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Remove /Users/user/Documents/handoffs" }));
    await expect(args.onRemove).toHaveBeenCalledWith("/Users/user/Documents/handoffs");
  },
};

export const NoneYet: Story = { name: "None yet", args: { allowed: [], blocked: [] } };

export const NotConnected: Story = { name: "Not connected", args: { disabled: true } };
