import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { WorkflowStacked, type WorkflowStackedRow } from "./WorkflowStacked";

const meta: Meta<typeof WorkflowStacked> = {
  title: "Compositions/Workflow stacked",
  component: WorkflowStacked,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-run-column)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WorkflowStacked>;

/** The run's steps, one per row, as the Workflow tab hands them in. */
const rows: WorkflowStackedRow[] = [
  { id: "step:plan", card: { kind: "step", name: "Plan the change", activity: "advanced", said: "advanced", ordinal: 1, line: "6m 00s · advanced", onOpen: fn() } },
  { id: "step:implement", card: { kind: "step", name: "Implement", activity: "running", said: "running", ordinal: 2, current: true, line: "55m · 4 groups", onOpen: fn() } },
  { id: "step:tests", card: { kind: "step", name: "Write tests", activity: "not_started", said: "not started", ordinal: 3, line: "not started", onOpen: fn() } },
  { id: "step:handoff", card: { kind: "step", name: "Review the change", activity: "not_started", said: "not started", ordinal: 4, line: "not started", gate: "will ask you", onOpen: fn() } },
];

/** The same feature run the canvas draws, as a column. */
export const AFeatureRun: Story = { args: { label: "The run", rows } };

/** A loop has nothing to arc over in a column, so the step says where it goes back to. */
export const WithALoop: Story = {
  args: {
    label: "The run",
    rows: rows.map((row) =>
      row.id === "step:tests"
        ? { ...row, returns: { toName: "Plan the change", label: "up to 5 passes" } }
        : row,
    ),
  },
};

/**
 * The claim the toggle rests on: the two arrangements say the same thing.
 *
 * **A `play`, because a still cannot say it.** The column carries the same
 * cards the canvas places — a column that dropped one would make the toggle a
 * change of subject.
 */
export const SaysWhatTheCanvasSays: Story = {
  args: { label: "The run", rows },
  play: async ({ canvas }) => {
    const run = canvas.getByRole("list", { name: "The run" });
    await expect(run).toBeVisible();
    for (const name of ["Plan the change, advanced", "Implement, running", "Write tests, not started"]) {
      await expect(canvas.getByRole("button", { name })).toBeVisible();
    }
  },
};
