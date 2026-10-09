import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { MorningReview } from "./MorningReview";

/**
 * What a night of sleep mode left, on its sheet: what was decided for the
 * owner, what still needs him, what landed and the walks held for him. Each
 * section is drawn only when it holds something, and none carries a count.
 *
 * The sheet lays out inside the nearest positioned ancestor, so every story
 * draws one.
 */
const meta: Meta<typeof MorningReview> = {
  title: "Compositions/Morning review",
  component: MorningReview,
  args: {
    open: true,
    decided: [
      { id: "d1", who: "Order the lunch", asked: "Which size?", chose: "Large" },
      { id: "d2", who: "Job 44", asked: "The check timed out twice", chose: "Retried with the larger timeout" },
    ],
    blocked: [{ id: "b1", who: "Job 52", text: "Pull request #2044 waits on your approval" }],
    landed: [{ id: "l1", who: "Job 41", title: "Retire the sleep calls", pr: "#2031" }],
    walks: [{ id: "w1", who: "Job 47", title: "Dispatch panel spacing", onOpen: fn() }],
    onOverride: fn(),
    onClose: fn(),
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

type Story = StoryObj<typeof MorningReview>;

/** All four sections. Override opens a field on its row and sends what was typed. */
export const TheNight: Story = {
  name: "A night's review",
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("dialog", { name: "Morning review" })).toBeVisible();
    for (const name of ["Decided for you", "Still needs you", "Landed overnight", "Walks waiting"]) {
      await expect(canvas.getByRole("region", { name })).toBeVisible();
    }
    await userEvent.click(canvas.getAllByRole("button", { name: "Override" })[0]!);
    const field = canvas.getByRole("textbox", { name: "Correction" });
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
    await userEvent.type(field, "Medium");
    await userEvent.click(canvas.getByRole("button", { name: "Send" }));
    await expect(args.onOverride).toHaveBeenCalledWith("d1", "Medium");
  },
};

/** An overridden row shows what was sent in place of the press. */
export const Corrected: Story = {
  name: "A corrected answer",
  args: {
    decided: [{ id: "d1", who: "Order the lunch", asked: "Which size?", chose: "Large", corrected: "Medium" }],
    blocked: [],
    landed: [],
    walks: [],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Medium")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Override" })).toBeNull();
    await expect(canvas.queryByRole("region", { name: "Still needs you" })).toBeNull();
  },
};
