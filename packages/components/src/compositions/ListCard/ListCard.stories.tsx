import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleDot, GitPullRequest } from "lucide-react";
import { expect, within } from "storybook/test";

import { ListCard, ListCardChip, ListCardMark, ListCardPending, ListCardTime } from "./ListCard";

/**
 * One thing on a list: a Job, an issue, a pull request or a Logbook entry. The title takes the room, the marks hold the top right corner with the state last, and the time is plain
 * text on the tag's line.
 */
const meta: Meta<typeof ListCard> = {
  title: "Compositions/List card",
  component: ListCard,
  decorators: [
    (Story) => (
      <ul style={{ width: "20rem", padding: 0, listStyle: "none", background: "var(--surface-canvas)" }}>
        <li>
          <Story />
        </li>
      </ul>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ListCard>;

const AT = "2026-10-20T09:00:00.000Z";

const MARKS = (
  <>
    <ListCardMark />
    <ListCardMark wide>
      <CircleDot size={16} aria-hidden="true" />
    </ListCardMark>
  </>
);

/** A Job: nothing under its title, so one line with the time before the marks. */
export const OneLine: Story = {
  args: { title: "Round the cart totals", marks: MARKS, at: <ListCardTime at={AT} said="Last turn" /> },
};

/** A pull request: its number under the title, and the time at the end of that line. */
export const WithTag: Story = {
  args: {
    title: "Fix the coupon expiry",
    marks: (
      <>
        <ListCardMark />
        <ListCardMark wide>
          <GitPullRequest size={16} aria-hidden="true" />
        </ListCardMark>
      </>
    ),
    tag: <ListCardChip>#3150</ListCardChip>,
    at: <ListCardTime at={AT} said="Last update" />,
  },
};

/** An issue: its title may take a second line. */
export const WrappedTitle: Story = {
  args: {
    wrap: true,
    title: "The Cockpit list drops a row while a Job is moving between repositories on the deck",
    marks: MARKS,
    tag: <ListCardChip>#2301</ListCardChip>,
    at: <ListCardTime at={AT} said="Last update" />,
  },
};

/** A line under the tag, as the person's View menu has it. */
export const WithLines: Story = {
  args: { title: "Prorate mid cycle plans", marks: MARKS, tag: <ListCardChip>#1892</ListCardChip>, lines: "Reading the billing tests" },
};

/** The one open beside the list, which says so to a reader as well as in its weight. */
export const Selected: Story = {
  args: { title: "Rotate the staging certificates", marks: MARKS, selected: true },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("button", { name: "Rotate the staging certificates" })).toHaveAttribute("aria-current", "true");
  },
};

/** Needs him: the card's faint tint pulses. */
export const Needs: Story = {
  args: { title: "Round the cart totals", marks: MARKS, needs: true },
};

/** Before its list has answered. */
export const Pending: Story = {
  render: () => <ListCardPending width="60%" />,
};
