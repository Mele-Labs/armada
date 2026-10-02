import type { Meta, StoryObj } from "@storybook/react-vite";
import { RunTreeSkeleton, type RunTreeSkeletonStep } from "./RunTreeSkeleton";

/** The run before a Job's own read has answered, drawn from the Bug workflow. */
const meta: Meta<typeof RunTreeSkeleton> = {
  title: "Compositions/Run tree skeleton",
  component: RunTreeSkeleton,
};
export default meta;

type Story = StoryObj<typeof RunTreeSkeleton>;

const BUG: RunTreeSkeletonStep[] = [
  { id: "repro", label: "Reproduction" },
  { id: "root_cause", label: "Root cause" },
  { id: "fix", label: "Fix" },
  { id: "regression_verify", label: "Regression check" },
  { id: "consumers", label: "Check the consumers still compile" },
  { id: "land", label: "Land" },
];

/**
 * The run while it is read. The workflow's names are drawn, and each row's mark
 * and duration wait.
 */
export const Reading: Story = {
  render: () => <RunTreeSkeleton steps={BUG} current="fix" />,
};

/** The run while it is read, with no workflow to name its steps from either. */
export const ReadingUnnamed: Story = {
  render: () => <RunTreeSkeleton />,
};
