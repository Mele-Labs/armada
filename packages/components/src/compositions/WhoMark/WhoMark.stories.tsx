import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { WhoMark } from "./WhoMark";

/** Whose way a retro item got in: a Drone's, the owner's or Fleet's. */
const meta: Meta<typeof WhoMark> = {
  title: "Compositions/Who mark",
  component: WhoMark,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WhoMark>;

/** A bare glyph, named by its tooltip and by nothing drawn. */
export const Drone: Story = {
  name: "A retro item in a Drone's way",
  args: { who: "drone" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Drone" })).toBeVisible();
  },
};

export const Owner: Story = { name: "A retro item in your way", args: { who: "owner" } };

export const Fleet: Story = { name: "A retro item in Fleet's way", args: { who: "fleet" } };

export const Agent: Story = {
  name: "A retro item in a Session agent's way",
  args: { who: "agent" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Agent" })).toBeVisible();
  },
};
