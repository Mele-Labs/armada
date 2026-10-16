import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

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
export const Ringed: Story = {};

/** The target is down in the corner, so the card takes the top one. */
export const TargetInTheCorner: Story = {
  args: { box: { x: 900, y: 600, width: 160, height: 36 } },
};

/** Looked for and not yet found: the card alone. */
export const Seeking: Story = {
  args: { box: null },
};
