import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { WalkGuide } from "./WalkGuide";

/** The ring and card over a walked page, so each story draws its target in the canvas as the bar draws it over the page. */
const meta: Meta<typeof WalkGuide> = {
  title: "Compositions/Walk guide",
  component: WalkGuide,
  parameters: { layout: "fullscreen" },
  args: {
    name: "Findings page",
    at: 2,
    of: 5,
    said: "The count sits beside the title",
    box: { x: 120, y: 120, width: 220, height: 32 },
    top: 72,
    onFold: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof WalkGuide>;

/** The target ringed, and the card in the trailing bottom corner. */
export const Ringed: Story = {
  play: async ({ canvas, canvasElement, args }) => {
    await expect(canvas.getByRole("img", { name: "The target of this step" })).toBeVisible();
    const card = canvas.getByRole("status");
    await expect(card).toHaveTextContent("Findings page");
    await expect(card).toHaveTextContent("Step 2 of 5");
    await expect(card).toHaveTextContent("The count sits beside the title");
    await expect(card).toHaveAttribute("data-edge", "bottom");
    // A press on the page beside the card is a person reaching for it: the ring is put away.
    const beside = canvasElement.querySelector<HTMLElement>(".armada-walk-guide__catch");
    beside?.click();
    await expect(args.onFold).toHaveBeenCalledTimes(1);
  },
};

/** The target is down in the corner, so the card takes the top one. */
export const TargetInTheCorner: Story = {
  args: { box: { x: 900, y: 600, width: 160, height: 36 } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "The target of this step" })).toBeVisible();
    await expect(canvas.getByRole("status")).toHaveAttribute("data-edge", "top");
  },
};

/** Looked for and not yet found: the card alone. */
export const Seeking: Story = {
  args: { box: null },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("img", { name: "The target of this step" })).toBeNull();
    await expect(canvas.getByRole("status")).toHaveTextContent("Step 2 of 5");
  },
};
