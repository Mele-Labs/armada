import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { Stamp, StampSurface } from "./Stamp";

// A fixed moment, so the readings do not move under the registry.
const NOW = new Date(2026, 9, 19, 14, 6, 9).getTime();
const FOUR_MINUTES_AGO = NOW - 4 * 60_000;
const TWO_HOURS_AGO = NOW - 2 * 60 * 60_000;
const LAST_WEEK = NOW - 7 * 24 * 60 * 60_000;

const meta: Meta<typeof Stamp> = {
  title: "Primitives/Stamp",
  component: Stamp,
};
export default meta;

type Story = StoryObj<typeof Stamp>;

/** A clock first, in the format set in Settings. The tooltip is the age a press gives. */
export const Clock: Story = {
  args: { at: FOUR_MINUTES_AGO, now: NOW },
};

/** Where an age is what is read first, a press gives the clock. */
export const AgeFirst: Story = {
  args: { at: FOUR_MINUTES_AGO, now: NOW, first: "age" },
};

/** An earlier day keeps its day with Date off. */
export const EarlierDay: Story = {
  args: { at: LAST_WEEK, now: NOW },
};

/** A press on one stamp in a surface turns every stamp in it, and a second press turns them back. */
export const OneSurface: Story = {
  render: () => (
    <StampSurface>
      <ul>
        <li>
          <Stamp at={FOUR_MINUTES_AGO} now={NOW} />
        </li>
        <li>
          <Stamp at={TWO_HOURS_AGO} now={NOW} />
        </li>
      </ul>
    </StampSurface>
  ),
  play: async ({ canvas, userEvent }) => {
    const [first] = canvas.getAllByRole("button");
    await userEvent.click(first!);
    await expect(canvas.getAllByRole("button").map((one) => one.firstChild?.textContent)).toEqual(["4m ago", "2h ago"]);
    first!.focus();
    await userEvent.keyboard("{Enter}");
    await expect(canvas.getAllByRole("button").some((one) => one.firstChild?.textContent?.endsWith("ago"))).toBe(false);
  },
};
