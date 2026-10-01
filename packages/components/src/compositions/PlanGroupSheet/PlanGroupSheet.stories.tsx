import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { PlanGroupSheet } from "./PlanGroupSheet";

/**
 * One plan group, pressed on the Plan graph. The body is the list's own card
 * body, so what is drawn here is what `Compositions/Plan board` draws under a
 * group's head. The sheet lays out inside the nearest positioned ancestor, so
 * the story draws one.
 */
const meta: Meta<typeof PlanGroupSheet> = {
  title: "Compositions/Plan group sheet",
  component: PlanGroupSheet,
  args: {
    open: true,
    onClose: fn(),
    onOpenTask: fn(),
    add: { label: "Add task", onAdd: fn() },
    group: {
      id: "g3",
      ordinal: 3,
      state: "failed",
      says: "failed",
      scope: { root: "packages/screens/src/**" },
      shapeSays: "2 tasks, at the same time",
      concurrent: true,
      overlaps: [{ says: "Group 2 writes these files too", paths: ["packages/screens/src/Board.tsx"] }],
      tasks: [
        { id: "T5", title: "Draw what is running, in four lists", mark: "done", tier: "difficult", model: "opus" },
        { id: "T6", title: "Open a Drone's Job from its row", mark: "done", tier: "medium", model: "sonnet" },
      ],
      boundary: {
        checks: [{ name: "vitest", reads: "failed" }],
        tests: [{ id: "c-board", spec: "packages/screens/src/Board.test.tsx", reads: "not covered" }],
      },
    },
  },
  decorators: [
    (Story) => (
      <div style={{ position: "relative", height: "var(--palette-max-height)", background: "var(--bg-base)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof PlanGroupSheet>;

/** A failed group: its state in the head, Add task beside Close, its tasks and boundary below. */
export const Failed: Story = {
  play: async ({ canvasElement, args }) => {
    const sheet = within(canvasElement).getByRole("dialog", { name: "Group 3" });
    await expect(within(sheet).getByText("failed at its checks")).toBeVisible();
    within(sheet).getByRole("button", { name: "Add task" }).click();
    await expect(args.add?.onAdd).toHaveBeenCalledWith("g3");
  },
};
