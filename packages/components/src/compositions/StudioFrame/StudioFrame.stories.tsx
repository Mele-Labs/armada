import type { Meta, StoryObj } from "@storybook/react-vite";

import { StudioFrame } from "./StudioFrame";

/**
 * A frame fills the box the whiteboard sizes round what it holds, so each
 * story gives it one. The whiteboard's own stories, and the read-in walk on
 * the mock, draw it round real nodes.
 */
const meta: Meta<typeof StudioFrame> = {
  title: "Compositions/Studio frame",
  component: StudioFrame,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-studio-node)", height: "calc(var(--w-studio-node) * 0.75)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof StudioFrame>;

/** A Zone: a well, headed by its kind, with no words of its own. */
export const Zone: Story = { args: { kind: "zone" } };

/** A Cluster: a dashed box headed by its title, the way it sits inside a read-in's Zone. */
export const Cluster: Story = { args: { kind: "cluster", title: "Risks to watch" } };

/** Picked on the board. */
export const Selected: Story = { args: { kind: "zone", selected: true } };
