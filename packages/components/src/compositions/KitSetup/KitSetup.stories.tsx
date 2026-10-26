import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SharedHomeBar } from "./SharedHome";

const meta: Meta<typeof SharedHomeBar> = {
  title: "Compositions/Kit setup",
  component: SharedHomeBar,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
  args: {
    home: { account: "user", repository: "armada-home", rows: [], syncing: false, choose: fn(), add: fn(), bring: fn(), sync: fn() },
  },
};
export default meta;

type Story = StoryObj<typeof SharedHomeBar>;

export const AtRest: Story = {
  play: async ({ args, canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Sync" }));
    await expect(args.home.sync).toHaveBeenCalled();
  },
};

export const Syncing: Story = { args: { home: { account: "user", repository: "armada-home", rows: [], syncing: true, choose: fn(), add: fn(), bring: fn(), sync: fn() } } };

/** A home Fleet has not made: the owner goes on press, and the name once Enter puts the picker away. An empty name is the default. */
export const ChoosingWhereItIsMade: Story = {
  args: { home: { account: "user", login: "user", owners: ["mele-labs"], made: false, repository: "armada-home", rows: [], syncing: false, choose: fn(), add: fn(), bring: fn(), sync: fn(), setHome: fn() } },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Home: user/armada-home" }));
    await userEvent.click(canvas.getByRole("radio", { name: "mele-labs" }));
    await expect(args.home.setHome).toHaveBeenLastCalledWith("mele-labs", "armada-home");
    const name = canvas.getByRole("textbox", { name: "Name" });
    await userEvent.clear(name);
    await userEvent.type(name, "team-home{Enter}");
    await expect(args.home.setHome).toHaveBeenLastCalledWith("user", "team-home");
    await expect(args.home.setHome).toHaveBeenCalledTimes(2);
  },
};
