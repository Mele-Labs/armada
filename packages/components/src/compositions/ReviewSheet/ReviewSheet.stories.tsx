import type { Meta, StoryObj } from "@storybook/react-vite";
import { AppWindow } from "lucide-react";
import { expect, fn, waitFor } from "storybook/test";

import { REVIEW_MARKS, ReviewSheet, type ReviewItem } from "./ReviewSheet";

/**
 * Everything one Job waits on, in one sheet: the list, oldest first, and the picked item drawn beside it. The head carries Send back and
 * the primary, which names the item it covers. The host decides what an answer does and tells the sheet whether the item is open.
 */
const meta: Meta<typeof ReviewSheet> = {
  title: "Compositions/Review sheet",
  component: ReviewSheet,
  args: {
    open: true,
    job: "Retry backoff cap",
    keep: "story:review-sheet",
    onPrimary: fn(),
    onSendBack: fn(),
    onClose: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof ReviewSheet>;

const ITEMS: ReviewItem[] = [
  {
    id: "plan",
    mark: REVIEW_MARKS.plan,
    name: "Plan v2",
    state: "open",
    answer: {
      verb: "Approve",
      sendBack: true,
    },
    body: <p>Cap the retry delay at thirty seconds.</p>,
  },
  {
    id: "doc",
    mark: REVIEW_MARKS.document,
    name: "Retry design",
    state: "open",
    answer: {
      verb: "Approve",
      sendBack: true,
    },
    body: <p>The cap is read once, when Fleet starts.</p>,
  },
  {
    id: "walk",
    mark: {
      Glyph: AppWindow,
      said: "Walk",
    },
    name: "Backoff table",
    state: "open",
    body: <p>The walk plays here, with its own bar.</p>,
  },
];

/** The oldest item is drawn first, and the head's primary names it. */
export const Open: Story = {
  args: {
    items: ITEMS,
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("region", {
        name: "Plan v2",
      }),
    ).toBeVisible();
    await expect(
      canvas.getByRole("button", {
        name: /^Approve Plan v2/,
      }),
    ).toBeVisible();
  },
};

/** A walk plays in the pane and has its own bar, so the head offers no Approve and no Send back for it. */
export const AWalkHasNoAnswerInTheHead: Story = {
  args: {
    items: ITEMS,
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(
      canvas.getByRole("button", {
        name: /Backoff table/,
      }),
    );
    await expect(
      canvas.getByRole("region", {
        name: "Backoff table",
      }),
    ).toBeVisible();
    await expect(
      canvas.queryByRole("button", {
        name: /^Approve/,
      }),
    ).toBeNull();
    await expect(
      canvas.queryByRole("button", {
        name: /^Send back/,
      }),
    ).toBeNull();
  },
};

/** The primary names what it approves, and ⌘↵ is the same press. Neither reaches any other item. */
export const PrimaryNamesItem: Story = {
  args: {
    items: ITEMS,
  },
  play: async ({ args, canvas, userEvent }) => {
    // The plan is the oldest, so it is already the one drawn.
    const approve = canvas.getByRole("button", {
      name: /^Approve Plan v2/,
    });
    await expect(approve).toBeVisible();
    await userEvent.click(approve);
    await expect(args.onPrimary).toHaveBeenCalledWith("plan");
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
    await expect(args.onPrimary).toHaveBeenCalledTimes(2);
    await expect(args.onPrimary).not.toHaveBeenCalledWith("doc");
  },
};

/** ⌥R swaps the body's foot for a note, and ⌘↵ in it sends the item back with his words, which is not an approval. */
export const SendBackOpensNote: Story = {
  args: {
    items: ITEMS,
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(
      canvas.getByRole("button", {
        name: /Retry design/,
      }),
    );
    await userEvent.keyboard("{Alt>}r{/Alt}");
    const note = canvas.getByRole("textbox", {
      name: "Note",
    });
    await waitFor(() => expect(note).toHaveFocus());
    await userEvent.type(note, "Say why.");
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
    await expect(args.onSendBack).toHaveBeenCalledWith("doc", "Say why.");
    await expect(args.onPrimary).not.toHaveBeenCalled();
  },
};

/** An item that is answered stays in the list, muted and marked, and the next open one is drawn. */
export const AnsweredStays: Story = {
  args: {
    items: ITEMS.map((one) =>
      one.id === "plan"
        ? {
            ...one,
            state: "answered" as const,
            said: "Approved",
          }
        : one,
    ),
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("img", {
        name: "Approved",
      }),
    ).toBeVisible();
  },
};

/** A body that has not arrived draws its shape in its place. */
export const Loading: Story = {
  args: {
    items: ITEMS.map((one) => ({
      ...one,
      loading: true,
    })),
  },
};
