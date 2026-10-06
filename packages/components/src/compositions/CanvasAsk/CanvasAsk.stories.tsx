import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { CanvasAsk } from "./CanvasAsk";

/** A card beside a canvas node that asks about it. The title and the box are the caller's. */
const meta: Meta<typeof CanvasAsk> = {
  title: "Compositions/Canvas ask",
  component: CanvasAsk,
};
export default meta;

type Story = StoryObj<typeof CanvasAsk>;

export const Asking: Story = {
  args: {
    title: "A Drone wants to run a command",
    children: <p>Wants to run pnpm add -D reselect@5.1.1</p>,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("group", { name: "Needs you" })).toBeVisible();
  },
};
