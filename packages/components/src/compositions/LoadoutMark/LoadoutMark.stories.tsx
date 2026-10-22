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
    await expect(within(card).getByText("pull_request_read × 40").nextElementSibling).toHaveTextContent("forge");

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

const AT = "2026-10-17T14:09:00.000Z";
const mcp = (name: string, from: string) => ({ turn_started_at: AT, kind: "tool" as const, name, source: "mcp" as const, from });

/**
 * **A long tool name stays inside the card.** An MCP tool is listed by its own name, since its value already names
 * the server, and a name that is still long wraps in its column rather than pushing the value out of the card.
 */
export const LongToolNames: Story = {
  args: {
    summary: {
      knows: 427,
      told: 3,
      latest_turn: {
        started_at: AT,
        used: [
          mcp("mcp__mailroom_docs__create_inspiration_document", "mailroom_docs"),
          mcp("mcp__mailroom_docs__create_inspiration_document_from_template_draft", "mailroom_docs"),
          mcp("mcp__browser-devtools__resize_page", "browser-devtools"),
        ],
      },
    },
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.hover(canvas.getByRole("button", { name: "Loadout" }));
    const card = (await screen.findByRole("tooltip")).firstElementChild!;
    const edge = card.getBoundingClientRect().right;
    for (const fact of card.querySelectorAll("dt, dd")) {
      const box = fact.getBoundingClientRect();
      await expect(box.right).toBeLessThanOrEqual(edge);
      // A value squeezed to nothing breaks one character a line; a server name takes one line or two. Its text is
      // measured, since a short value in a tall row is as tall as the row.
      if (fact.tagName === "DD" && fact.textContent !== "") {
        const text = document.createRange();
        text.selectNodeContents(fact);
        await expect(text.getBoundingClientRect().height).toBeLessThan(2.5 * parseFloat(getComputedStyle(fact).lineHeight));
      }
    }
    await expect(within(card as HTMLElement).getByText("create_inspiration_document")).toBeInTheDocument();
    await expect(within(card as HTMLElement).queryByText(/^mcp__/)).not.toBeInTheDocument();
  },
};
