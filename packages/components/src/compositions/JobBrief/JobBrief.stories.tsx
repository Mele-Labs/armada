import type { Meta, StoryObj } from "@storybook/react-vite";

import { JobBriefSkeleton } from "./JobBrief";

/**
 * The brief while it is read. The brief itself is drawn through `Prose` on
 * Overview's board; this is what stands in for it until the read answers.
 */
const meta: Meta<typeof JobBriefSkeleton> = {
  title: "Compositions/Job brief",
  component: JobBriefSkeleton,
};
export default meta;

type Story = StoryObj<typeof JobBriefSkeleton>;

/** The brief while it is read — two lines, what a brief usually wraps to. */
export const Reading: Story = {
  render: () => <JobBriefSkeleton />,
};
