import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { JobCard } from "./JobCard";

/** A Job as a card, framed in its state: the glyph and its tooltip, the last line it said, its age and its repository. */
const meta: Meta<typeof JobCard> = {
  title: "Compositions/Job card",
  component: JobCard,
  args: {
    title: "Fix the coupon expiry check",
    state: "working",
    said: "Working",
    says: "Running the checkout tests",
    at: "2026-10-23T09:12:00Z",
    repository: "storefront",
    onOpen: fn(),
  },
  decorators: [(Story) => <div style={{ width: 320 }}><Story /></div>],
};
export default meta;

type Story = StoryObj<typeof JobCard>;

export const Working: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "Working" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Job Fix the coupon expiry check" }));
    await expect(args.onOpen).toHaveBeenCalledOnce();
  },
};

export const WaitingOnYou: Story = { args: { title: "Renew the edge certificates", state: "waiting", said: "Waiting on you", says: "Which certificate authority?" } };

export const Failing: Story = { args: { title: "Prorate a mid-cycle upgrade", state: "failing", said: "Checks failed on #412", says: "invoice_totals failed" } };

export const Idle: Story = { args: { title: "Round the cart totals", state: "idle", said: "Idle", says: "Pull request open" } };

export const Selected: Story = { args: { selected: true } };

/** On a surface that already names the repository, the chip is left out. */
export const NoRepository: Story = { args: { repository: undefined } };
