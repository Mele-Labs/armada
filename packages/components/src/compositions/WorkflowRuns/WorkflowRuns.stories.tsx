import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { FiredRuns, RunRows } from "./WorkflowRuns";
import { MOCK_RUNS, resolveRuns } from "./runs";

const meta: Meta<typeof RunRows> = {
  title: "Compositions/Workflow runs",
  component: RunRows,
  args: { runs: resolveRuns(MOCK_RUNS), label: "Runs", onOpen: fn() },
};
export default meta;

type Story = StoryObj<typeof RunRows>;

/** Each run marked by where it is set, and the repository's replaced by the machine's drawn quiet. */
export const Rows: Story = {
  play: async ({ canvasElement }) => {
    const rows = within(canvasElement);
    await expect(rows.getByRole("button", { name: /deploy_qa, PR opened, this repository, overridden/ })).toBeVisible();
  },
};

/** Runs that fired: one passed, one failed with a repair Drone working. */
export const Fired: Story = {
  render: () => (
    <FiredRuns
      runs={[
        { name: "lint_docs", when: "implement passes", place: "repository", state: "passed" },
        { name: "deploy_qa", when: "PR opened", place: "kit", state: "repairing" },
      ]}
    />
  ),
};
