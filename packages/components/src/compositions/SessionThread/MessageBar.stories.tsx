import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import type { QuickResponse } from "../../primitives/Prose/responses";
import { MessageBar } from "./MessageBar";

/**
 * What sits under an agent's message in the spacious take: Copy, Reply with the message quoted, the
 * quick responses, and when it was said. Reply is there only where a host can take it.
 */
const RESPONSES: QuickResponse[] = [
  {
    id: "pick",
    emoji: "✅",
    name: "Pick this",
    kind: "send",
  },
  {
    id: "yes",
    emoji: "👍",
    name: "Yes to this",
    kind: "send",
  },
  {
    id: "why",
    emoji: "❓",
    name: "Why this",
    kind: "ask",
  },
];

const meta: Meta<typeof MessageBar> = {
  title: "Compositions/Session thread/Message bar",
  component: MessageBar,
  args: {
    text: "The test reads the wall clock.",
    responses: RESPONSES,
    onReply: fn(),
    onPick: fn(),
    at: <span>14:03</span>,
  },
};
export default meta;

type Story = StoryObj<typeof MessageBar>;

/** Copy, Reply, one button per response and the time, in a toolbar named for the message. */
export const Whole: Story = {
  play: async ({ canvas }) => {
    const bar = canvas.getByRole("toolbar", {
      name: "Message",
    });
    await expect(bar).toContainElement(
      canvas.getByRole("button", {
        name: "Copy message",
      }),
    );
    await expect(
      canvas.getByRole("button", {
        name: "Reply",
      }),
    ).toBeVisible();
    for (const one of RESPONSES)
      await expect(
        canvas.getByRole("button", {
          name: one.name,
        }),
      ).toBeVisible();
    await expect(bar).toHaveTextContent("14:03");
  },
};

/** Reply and a response each call their own handler: Reply with nothing, the response with the one pressed. */
export const Presses: Story = {
  play: async ({ canvas, args }) => {
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Reply",
      }),
    );
    await expect(args.onReply).toHaveBeenCalledTimes(1);
    await expect(args.onPick).not.toHaveBeenCalled();
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Yes to this",
      }),
    );
    await expect(args.onPick).toHaveBeenCalledTimes(1);
    await expect(args.onPick).toHaveBeenCalledWith(RESPONSES[1]);
    await expect(args.onReply).toHaveBeenCalledTimes(1);
  },
};

/** Only the response he already sent is pressed. */
export const Marked: Story = {
  args: {
    marked: "👍",
  },
  play: async ({ canvas }) => {
    for (const one of RESPONSES)
      await expect(
        canvas.getByRole("button", {
          name: one.name,
        }),
      ).toHaveAttribute("aria-pressed", String(one.id === "yes"));
  },
};

/** No `onReply`, no Reply: the act is left off, and the rest of the bar stays. */
export const WithoutReply: Story = {
  args: {
    onReply: undefined,
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.queryByRole("button", {
        name: "Reply",
      }),
    ).toBeNull();
    await expect(
      canvas.getByRole("button", {
        name: "Copy message",
      }),
    ).toBeVisible();
    await expect(
      canvas.getByRole("button", {
        name: "Pick this",
      }),
    ).toBeVisible();
  },
};
