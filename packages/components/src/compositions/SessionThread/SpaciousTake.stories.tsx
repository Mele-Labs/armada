import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { DEFAULT_RESPONSES } from "../../primitives/Prose/responses";
import { SessionThread, type SessionThreadRow } from "./SessionThread";
import { SpaciousTake } from "./spacious-take";

/**
 * A Job's thread in the spacious take, which only the Job view provides: an agent's message carries one bar
 * under all of it, with no bar on its lines, and the slot a Job holds reads as a line. Drawn without the take, a
 * thread stays as it was.
 */
const meta: Meta<typeof SessionThread> = {
  title: "Compositions/Session thread/Spacious take",
  component: SessionThread,
  args: {
    rows: [],
    onAnswer: fn(),
    onOpenSession: fn(),
    onReplyAt: fn(),
    onPickLine: fn(),
    responses: DEFAULT_RESPONSES,
  },
  decorators: [
    (Story) => (
      <div
        style={{
          display: "flex",
          height: "calc(var(--space-12) * 8)",
          width: "calc(var(--space-12) * 10)",
          background: "var(--bg-raised)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionThread>;

const take: NonNullable<Story["decorators"]> = [
  (Story) => (
    <SpaciousTake.Provider value={true}>
      <Story />
    </SpaciousTake.Provider>
  ),
];

const SAID: SessionThreadRow = {
  id: "m1",
  at: "14:03:28",
  kind: "message",
  from: "agent",
  text: "The test reads the wall clock.\n\nPin it before the assertion.",
};

const SLOT: SessionThreadRow[] = [
  {
    id: "l1",
    at: "14:03:21",
    kind: "lease",
    slot: 3,
    branch: "fix/flaky-store",
  },
  {
    id: "l2",
    at: "14:41:02",
    kind: "release",
    slot: 3,
    branch: "fix/flaky-store",
  },
];

/** The message has its bar, and its lines have none: the bar is the only place to answer it. */
export const MessageHasABar: Story = {
  decorators: take,
  args: {
    rows: [
      SAID,
    ],
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("toolbar", {
        name: "Message",
      }),
    ).toBeInTheDocument();
    await expect(
      canvas.queryAllByRole("group", {
        name: /^Quick responses:/,
        hidden: true,
      }),
    ).toHaveLength(0);
  },
};

/** Reply hands the host the row and the whole message as a markdown quote, each line behind `> `. */
export const ReplyQuotesTheMessage: Story = {
  decorators: take,
  args: {
    rows: [
      SAID,
    ],
  },
  play: async ({ canvas, args }) => {
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Reply",
      }),
    );
    await expect(args.onReplyAt).toHaveBeenCalledTimes(1);
    await expect(args.onReplyAt).toHaveBeenCalledWith({
      rowId: "m1",
      quote: "> The test reads the wall clock.\n>\n> Pin it before the assertion.\n\n",
    });
  },
};

/** A response on the bar answers the whole message, through the thread's own `onPickLine`. */
export const BarAnswersTheMessage: Story = {
  decorators: take,
  args: {
    rows: [
      SAID,
    ],
  },
  play: async ({ canvas, args }) => {
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Yes to this",
      }),
    );
    await expect(args.onPickLine).toHaveBeenCalledTimes(1);
    await expect(args.onPickLine).toHaveBeenCalledWith(
      "The test reads the wall clock. Pin it before the assertion.",
      DEFAULT_RESPONSES.find((one) => one.id === "yes"),
    );
  },
};

/** Without the take the message has no bar, and its lines answer one by one as they always have. */
export const WithoutTheTake: Story = {
  args: {
    rows: [
      SAID,
    ],
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.queryByRole("toolbar", {
        name: "Message",
      }),
    ).toBeNull();
    await expect(
      canvas.queryByRole("button", {
        name: "Reply",
        hidden: true,
      }),
    ).toBeNull();
    await expect(
      canvas.getAllByRole("group", {
        name: /^Quick responses:/,
        hidden: true,
      }).length,
    ).toBeGreaterThan(0);
  },
};

/** The slot leased and given back are each one quiet line, marked as a line of the thread. */
export const SlotIsALine: Story = {
  decorators: take,
  args: {
    rows: SLOT,
  },
  play: async ({ canvas }) => {
    const leased = canvas.getByRole("region", {
      name: "Slot leased",
    });
    await expect(leased).toHaveAttribute("data-line");
    await expect(leased).toHaveTextContent("Leased slot 3");
    await expect(leased).toHaveTextContent("fix/flaky-store");
    const released = canvas.getByRole("region", {
      name: "Slot released",
    });
    await expect(released).toHaveAttribute("data-line");
    await expect(released).toHaveTextContent("Released slot 3");
  },
};

/** Without the take the slot stays a card, headed by its eyebrow and not drawn as a line. */
export const SlotStaysACard: Story = {
  args: {
    rows: SLOT,
  },
  play: async ({ canvas }) => {
    const leased = canvas.getByRole("region", {
      name: "Slot leased",
    });
    await expect(leased).not.toHaveAttribute("data-line");
    await expect(leased).not.toHaveTextContent("Leased slot 3");
  },
};
