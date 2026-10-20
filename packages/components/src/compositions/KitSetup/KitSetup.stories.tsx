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
