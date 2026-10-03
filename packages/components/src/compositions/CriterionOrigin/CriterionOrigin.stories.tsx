import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor } from "storybook/test";

import { CriterionMovedMark, CriterionOriginMark } from "./CriterionOrigin";

/** One story per origin, and the issue having moved: the whole vocabulary. */
const meta: Meta<typeof CriterionOriginMark> = {
  title: "Compositions/Criterion origin",
  component: CriterionOriginMark,
};
export default meta;

type Story = StoryObj<typeof CriterionOriginMark>;

/**
 * **A bare mark names itself on hover.** Nothing beside it prints where the
 * words came from, so the tooltip is the only place a pointer reads it.
 */
export const FromAnIssue: Story = {
  args: { origin: "issue" },
  play: async ({ canvas, userEvent }) => {
    const mark = canvas.getByRole("img", { name: "From an issue" });
    await expect(canvas.getByText("From an issue")).not.toBeVisible();
    await userEvent.hover(mark);
    await waitFor(() => expect(canvas.getByText("From an issue")).toBeVisible());
  },
};
export const FromTheRequest: Story = { args: { origin: "prompt" } };
export const ByAPerson: Story = { args: { origin: "person" } };

/** The issue edited since Fleet read it: the date is in the name, and nowhere on the line. */
export const TheIssueMoved: Story = {
  render: () => <CriterionMovedMark at="2 Oct 2026 at 15:41" />,
  play: async ({ canvas, userEvent }) => {
    const name = "The issue has been edited since Fleet read it, last on 2 Oct 2026 at 15:41";
    await userEvent.hover(canvas.getByRole("img", { name }));
    await waitFor(() => expect(canvas.getByText(name)).toBeVisible());
  },
};
