import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { StopCard, StopCardList } from "./StopCard";

/**
 * The card a metro stop opens in the Bays view and a Session's context figure opens: a toned pill and a
 * subtle place over a title, then facts. The tone is `--tone`, set by whoever draws it.
 */
const meta: Meta<typeof StopCard> = {
  title: "Compositions/Stop card",
  component: StopCard,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof StopCard>;

/** A pill, a place, a title and facts, the values in mono. */
export const Facts: Story = {
  args: {
    role: "dialog",
    "aria-label": "5-hour limit",
    style: { ["--tone" as string]: "var(--fg-muted)" },
    pill: "34% used",
    where: "5h",
    title: "5-hour limit",
    rows: [["Resets in", "2h 14m", true]],
  },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("dialog", { name: "5-hour limit" });
    await expect(card).toHaveTextContent("34% used");
    await expect(canvas.getByText("Resets in")).toBeVisible();
    await expect(canvas.getByText("2h 14m")).toBeVisible();
  },
};

/** What a caller puts after the facts: a titled list, and none where the list is empty. */
export const Lists: Story = {
  args: {
    role: "dialog",
    "aria-label": "Context",
    style: { ["--tone" as string]: "var(--status-awaiting-review)" },
    pill: "94% used",
    where: "188k / 200k",
    title: "Context",
    rows: [["Messages", "158k", true]],
    children: (
      <>
        <StopCardList heading="Memory files" rows={[["NOTES.md", "2.1k", true]]} />
        <StopCardList heading="MCP tools" rows={[]} />
      </>
    ),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Memory files")).toBeVisible();
    await expect(canvas.getByText("NOTES.md")).toBeVisible();
    await expect(canvas.queryByText("MCP tools")).not.toBeInTheDocument();
  },
};
