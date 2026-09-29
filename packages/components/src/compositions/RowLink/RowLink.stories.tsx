import type { Meta, StoryObj } from "@storybook/react-vite";
import { ShieldCheck, ShieldX } from "lucide-react";
import { expect, fn, userEvent } from "storybook/test";

import { TaskMark } from "../TaskMark/TaskMark";
import { RowLink } from "./RowLink";

const meta: Meta<typeof RowLink> = {
  title: "Compositions/Row link",
  component: RowLink,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-step-panel-min)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof RowLink>;

/** A Check that failed, opening its own Record row. Named by what it came to. */
export const FailedCheck: Story = {
  args: {
    mark: <ShieldX size={12} strokeWidth={2} />,
    children: "screens_test",
    mono: true,
    says: "failed",
    tone: "failed",
    label: "screens_test, failed",
    onOpen: fn(),
  },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "screens_test, failed" }));
    await expect(args.onOpen).toHaveBeenCalledOnce();
  },
};

/** A Check with no row of its own to open: the same line, and not a button. */
export const NothingToOpen: Story = {
  args: {
    mark: <ShieldCheck size={12} strokeWidth={2} />,
    children: "typecheck",
    mono: true,
    says: "passed",
    tone: "passed",
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button")).toBeNull();
  },
};

/** A task a failed Check held back, opening the task's Record row. */
export const Task: Story = {
  args: {
    mark: <TaskMark state="failed" />,
    children: "T6 · Open a Drone's Job from its row",
    onOpen: fn(),
  },
};
