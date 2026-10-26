import type { Meta, StoryObj } from "@storybook/react-vite";

import { AdversarialReviewPress } from "./AdversarialReview";

/**
 * One story per state of the press: idle, running, done with findings, done with none, failed and stale. It knows nothing
 * of what was read: the host gives it a state and the callback. The findings are pinned to the plan, and their stories are
 * the plan card's (`SessionThread.stories.tsx`).
 */
const meta: Meta<typeof AdversarialReviewPress> = {
  title: "Compositions/Adversarial review",
  component: AdversarialReviewPress,
  args: { onRun: () => undefined, shortcut: "r" },
};
export default meta;

type Story = StoryObj<typeof AdversarialReviewPress>;

const FINDING = {
  id: "f1",
  consequence: "A lease waits ten minutes when every slot is held",
  expected: "The warm skips a held slot and tries the next",
  found: "It warms the slot the pool picked, held or not, and the lease waits on that warm to end",
};

export const Idle: Story = { args: { review: { state: "idle" } } };

export const Running: Story = { args: { review: { state: "running" } } };

export const DoneWithFindings: Story = { args: { review: { state: "done", findings: [FINDING] } } };

export const DoneWithNone: Story = { args: { review: { state: "done", findings: [] } } };

export const Failed: Story = { args: { review: { state: "failed", said: "The adversary stopped before it finished reading" } } };

export const Stale: Story = { args: { review: { state: "stale", read: 1, findings: [FINDING] } } };
