import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, within } from "storybook/test";

import { LoadoutMark } from "./LoadoutMark";
import { SUMMARY } from "../LoadoutSheet/LoadoutSheet.fixtures";

/** What a Session's agent carries, as a quiet mark in its head. */
const meta: Meta<typeof LoadoutMark> = {
  title: "Compositions/Loadout mark",
  component: LoadoutMark,
  args: { summary: SUMMARY, onOpen: fn() },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof LoadoutMark>;

/**
 * Hover opens the card: the counts, and what the latest turn used with where each came from. **A press opens the
 * whole Loadout and does not pin the card**, and the card's own Show all does the same.
 */
export const Reported: Story = {
  play: async ({ canvas, userEvent, args }) => {
    const mark = canvas.getByRole("button", { name: "Loadout" });
    await userEvent.hover(mark);
    const card = await screen.findByRole("tooltip");
    await expect(within(card).getByText("Knows").nextElementSibling).toHaveTextContent("19");
    await expect(within(card).getByText("armada:mechanic").nextElementSibling).toHaveTextContent("Armada · sonnet");
    await expect(within(card).getByText("mcp__forge__pull_request_read × 40").nextElementSibling).toHaveTextContent("forge");

    await userEvent.click(within(card).getByRole("button", { name: "Show all" }));
    await expect(args.onOpen).toHaveBeenCalledOnce();

    await userEvent.click(mark);
    await expect(args.onOpen).toHaveBeenCalledTimes(2);
    await expect(screen.queryByRole("dialog", { name: "Loadout" })).not.toBeInTheDocument();
  },
};

/** Nothing used yet: the counts alone. */
export const NothingUsed: Story = {
  args: { summary: { knows: 19, told: 3 } },
  play: async ({ canvas, userEvent }) => {
    await userEvent.hover(canvas.getByRole("button", { name: "Loadout" }));
    const card = await screen.findByRole("tooltip");
    await expect(within(card).queryByText("Latest turn")).not.toBeInTheDocument();
  },
};
