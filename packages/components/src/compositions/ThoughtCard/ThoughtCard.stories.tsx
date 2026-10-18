import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { ThoughtCard } from "./ThoughtCard";

/**
 * A thought typed into Helm's dock, drawn at the dock's own width on its sunken ground. One story
 * per stage: Helm still reading, a match with and without a slot, and the choice to start new.
 */
const meta: Meta<typeof ThoughtCard> = {
  title: "Compositions/Thought card",
  component: ThoughtCard,
  render: (args) => (
    <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
      <ThoughtCard {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof ThoughtCard>;

const WORDS = "I wish the sessions page would move sessions into a status that says they are waiting for work to land";

const onSend = fn();
const onNo = fn();
const onAskHelm = fn();
const onSession = fn();
const onDismiss = fn();

/** A Session that holds a slot: its name draws the Session's card, and Send, No and Ask Helm each carry their key. */
export const MatchSession: Story = {
  args: {
    stage: "match",
    words: WORDS,
    target: { kind: "session", title: "Sessions list views", repository: "armada", slot: 3 },
    onSend,
    onNo,
    onAskHelm,
  },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("group", { name: "Send to" })).toBeVisible();
    await expect(canvas.getByText(WORDS)).toBeVisible();
    await expect(canvas.getByText("Sessions list views")).toBeVisible();
    await expect(canvas.getByText("armada")).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Job" })).toBeVisible();

    // The key each act answers to is on its button and in the shortcuts the dock binds.
    const send = canvas.getByRole("button", { name: "Send" });
    await expect(send).toHaveTextContent("↵");
    await expect(send).toHaveAttribute("aria-keyshortcuts", "↵");
    await expect(canvas.getByRole("button", { name: "No" })).toHaveTextContent("N");
    await expect(canvas.getByRole("button", { name: "Ask Helm" })).toHaveTextContent("H");

    await userEvent.click(send);
    await expect(onSend).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "No" }));
    await expect(onNo).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "Ask Helm" }));
    await expect(onAskHelm).toHaveBeenCalledTimes(1);
  },
};

/** A Session that holds no slot: with the Session's glyph. */
export const MatchSessionWithoutSlot: Story = {
  args: {
    stage: "match",
    words: WORDS,
    target: { kind: "session", title: "Release notes script", repository: "armada" },
    onSend,
    onNo,
    onAskHelm,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Job" })).toBeVisible();
    await expect(canvas.getByText("Release notes script")).toBeVisible();
  },
};

/** Nothing matched, or No pressed: start a Session with the words, or ask Helm. Esc dismisses. */
export const New: Story = {
  args: {
    stage: "new",
    words: "the merge line page should group PRs by repository",
    onSession,
    onAskHelm,
    onDismiss,
  },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("group", { name: "Start new" })).toBeVisible();
    await expect(canvas.getByText("the merge line page should group PRs by repository")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Job" })).toHaveTextContent("S");
    await expect(canvas.getByRole("button", { name: "Ask Helm" })).toHaveTextContent("H");
    await expect(canvas.getByRole("button", { name: "Dismiss" })).toHaveAttribute("aria-keyshortcuts", "Escape");

    await userEvent.click(canvas.getByRole("button", { name: "Job" }));
    await expect(onSession).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "Ask Helm" }));
    await expect(onAskHelm).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "Dismiss" }));
    await expect(onDismiss).toHaveBeenCalledTimes(1);
  },
};
