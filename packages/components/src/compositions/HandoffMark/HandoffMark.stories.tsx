import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { EarlierMark, HandoffMark } from "./HandoffMark";

const meta: Meta<typeof HandoffMark> = {
  title: "Compositions/Handoff mark",
  component: HandoffMark,
};
export default meta;

type Story = StoryObj<typeof HandoffMark>;

/** A Job that asked to carry on in a new one: one turning ring on its row, named by its tooltip, until the new Job is up. */
export const HandingOff: Story = {};

/** The earlier Job behind a row that took its place. A press opens it. */
export const Earlier: Story = {
  render: () => <EarlierMark onOpen={fn()} />,
};
