import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { HeldSlots } from "./HeldSlots";

const NOW = Date.parse("2026-11-02T12:00:00Z");
const hoursAgo = (hours: number) => new Date(NOW - hours * 3_600_000).toISOString();

/** The slots a full pool is held by, under the call's choices: the real case, three slots held by Jobs that went quiet 288, 98 and 93 hours ago. */
const meta: Meta<typeof HeldSlots> = {
  title: "Compositions/Held slots",
  component: HeldSlots,
  args: {
    now: NOW,
    onOpen: fn(),
    onFree: fn(),
    onCreate: fn(),
    rows: [
      {
        slot: 4,
        job: {
          id: "j2",
          title: "Rework the logbook filters",
        },
        quietSince: hoursAgo(98),
      },
      {
        slot: 2,
        job: {
          id: "j1",
          title: "Move the run ledger",
        },
        quietSince: hoursAgo(288),
      },
      {
        slot: 5,
        job: {
          id: "j3",
          title: "Pocket pairing flow",
        },
        quietSince: hoursAgo(93),
      },
    ],
  },
  decorators: [
    (Story) => (
      <div
        style={{
          width: "calc(var(--space-12) * 8)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof HeldSlots>;

/** Longest quiet first whatever order they arrive in, each with Free on. */
export const Quiet: Story = {};

/** A running holder and one with uncommitted work keep their Free column, off, with the reason on hover. */
export const FreeOff: Story = {
  args: {
    rows: [
      {
        slot: 2,
        job: {
          id: "j1",
          title: "Move the run ledger",
        },
        quietSince: hoursAgo(288),
        keeps: "uncommitted",
      },
      {
        slot: 4,
        job: {
          id: "j2",
          title: "Rework the logbook filters",
        },
        quietSince: hoursAgo(1),
        keeps: "running",
      },
      {
        slot: 5,
        job: {
          id: "j3",
          title: "Pocket pairing flow",
        },
        quietSince: hoursAgo(93),
      },
    ],
  },
};

/** One Free is out: its row is pending and the others are held until Fleet answers. */
export const Freeing: Story = {
  args: {
    onFree: () => new Promise(() => undefined),
  },
  play: async ({ canvas }) => {
    canvas
      .getByRole("button", {
        name: "Free slot 2",
      })
      .click();
  },
};

/** Fleet is making a slot, which can take a while: the row counts up, and Free stays on. */
export const Creating: Story = {
  args: {
    creatingSince: new Date(NOW - 74_000).toISOString(),
  },
};

/** Fleet could not make it: its words under the row, and Create is back. */
export const CreateRefused: Story = {
  args: {
    createRefused: "no space left on the disk",
  },
};
