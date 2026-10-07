import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { FiredTriggers, TriggerRows } from "./WorkflowTriggers";
import { MOCK_TRIGGERS, resolveTriggers } from "./triggers";

const meta: Meta<typeof TriggerRows> = {
  title: "Compositions/Workflow triggers",
  component: TriggerRows,
  args: { triggers: resolveTriggers(MOCK_TRIGGERS), label: "Triggers", onOpen: fn() },
};
export default meta;

type Story = StoryObj<typeof TriggerRows>;

/** Each run marked by where it is set, and the repository's replaced by the machine's drawn quiet. */
export const Rows: Story = {
  play: async ({ canvasElement }) => {
    const rows = within(canvasElement);
    await expect(rows.getByRole("button", { name: /deploy_qa, PR opened, this repository, overridden/ })).toBeVisible();
  },
};

/** Triggers that fired: one passed, one failed with a repair Drone working. */
export const Fired: Story = {
  render: () => (
    <FiredTriggers
      triggers={[
        { name: "lint_docs", when: "implement passes", place: "repository", state: "passed" },
        { name: "deploy_qa", when: "PR opened", place: "kit", state: "repairing" },
      ]}
    />
  ),
};
