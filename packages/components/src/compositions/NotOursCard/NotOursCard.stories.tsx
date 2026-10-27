import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { NotOursCard } from "./NotOursCard";

const SUMMARY = [
  "provision failed in QA deploy.",
  "When: 2026-10-26 14:02 UTC",
  "Run: QA deploy",
  "Error: Cloud lookup for region qa-east returned no account.",
  "Link: https://ci.example.com/runs/7",
].join("\n");

/** A watch that ended not ours, as Needs you opens it: what failed with the agent's sentence, Fix it, Ignore and Escalate. */
const meta: Meta<typeof NotOursCard> = {
  title: "Compositions/Not ours card",
  component: NotOursCard,
  args: {
    watch: "QA deploy",
    parts: [{ name: "provision", sentence: "A cloud lookup outside this change failed.", error: "Error: cloud lookup for region qa-east returned no account" }],
    link: "https://ci.example.com/runs/7",
    summary: SUMMARY,
    onFixIt: fn(),
    onIgnore: fn(),
    onOpenLink: fn(),
  },
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 10)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof NotOursCard>;

/** Escalate shows the lines for the owners and one copy control, sends nothing, and leaves Fix it and Ignore to settle it. */
export const Escalated: Story = {
  play: async ({ canvas, args }) => {
    await expect(canvas.getByLabelText("provision failed on")).toHaveTextContent("returned no account");
    await expect(canvas.queryByRole("region", { name: "Summary for the owners" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Escalate" }));
    const summary = canvas.getByRole("region", { name: "Summary for the owners" });
    await expect(summary).toHaveTextContent("Error: Cloud lookup for region qa-east returned no account.");
    await expect(canvas.getAllByRole("button", { name: /^Copy/ })).toHaveLength(1);
    await expect(args.onFixIt).not.toHaveBeenCalled();
    await expect(args.onIgnore).not.toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "Fix it" }));
    await expect(args.onFixIt).toHaveBeenCalledOnce();
  },
};

/** Two parts set aside failed: both are named, each with its own sentence. */
export const TwoParts: Story = {
  args: {
    parts: [
      { name: "provision", sentence: "A cloud lookup outside this change failed." },
      { name: "dns", sentence: "The shared zone was being moved." },
    ],
  },
};
