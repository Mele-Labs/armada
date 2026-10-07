import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { FiredHooks, HookRows } from "./WorkflowHooks";
import { MOCK_HOOKS, resolveHooks } from "./hooks";

const meta: Meta<typeof HookRows> = {
  title: "Compositions/Workflow hooks",
  component: HookRows,
  args: { hooks: resolveHooks(MOCK_HOOKS), label: "Hooks", onOpen: fn() },
};
export default meta;

type Story = StoryObj<typeof HookRows>;

/** Each run marked by where it is set, and the repository's replaced by the machine's drawn quiet. */
export const Rows: Story = {
  play: async ({ canvasElement }) => {
    const rows = within(canvasElement);
    await expect(rows.getByRole("button", { name: /deploy_qa, PR opened, this repository, overridden/ })).toBeVisible();
  },
};

/** Hooks that fired: one passed, one failed with a repair Drone working. */
export const Fired: Story = {
  render: () => (
    <FiredHooks
      hooks={[
        { name: "lint_docs", when: "implement passes", place: "repository", state: "passed" },
        { name: "deploy_qa", when: "PR opened", place: "kit", state: "repairing" },
      ]}
    />
  ),
};
