import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { Bookmark, ExternalLink, Trash2, Type } from "lucide-react";

import { StudioPicked, type StudioPickedAct } from "./StudioPicked";

const meta: Meta<typeof StudioPicked> = {
  title: "Compositions/Studio picked",
  component: StudioPicked,
  decorators: [
    (Story) => (
      <div className="armada-graph-node-bar">
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof StudioPicked>;

/** Everything a Contradiction offers, with #1406's Open and the delete beside them. */
const EVERY_ACT: StudioPickedAct[] = [
  { id: "open", label: "Open", icon: ExternalLink },
  { id: "write_up", label: "Write up", icon: Type },
  { id: "defer", label: "Defer", icon: Bookmark },
  { id: "settled", label: "Not a problem" },
  { id: "resolved", label: "Resolved here" },
  { id: "remove", label: "Delete node", icon: Trash2, danger: true },
];

/** The order the row draws: glyphs, then the acts that must say their names, then the delete. */
const AS_DRAWN = ["Open", "Write up", "Defer", "Not a problem", "Resolved here", "Delete node"];

/**
 * One node offering six acts. **Every one is a press**, which is the owner's
 * correction of 28 Sep 2026 — the menu that stood here asked for two.
 */
export const OneNode: Story = {
  args: { acts: EVERY_ACT, onAct: fn() },
  play: async ({ canvas, args, userEvent, step }) => {
    await step("every act is on the bar, the delete last", async () => {
      const offered = canvas.getAllByRole("button").map((one) => one.getAttribute("aria-label"));
      await expect(offered).toEqual(AS_DRAWN);
    });

    await step("one press acts, with nothing opened first", async () => {
      await userEvent.click(canvas.getByRole("button", { name: "Resolved here" }));
      await expect(args.onAct).toHaveBeenCalledWith("resolved");
    });
  },
};

/** A Studio picked over whole, to delete it or to read it as one Outline. */
export const ManyNodes: Story = {
  args: {
    acts: [
      { id: "outline", label: "Outline" },
      { id: "remove", label: "Delete 40 nodes", icon: Trash2, danger: true },
    ],
    onAct: fn(),
  },
  play: async ({ canvas, step }) => {
    await step("how many are going, and not which", async () => {
      await expect(canvas.getByRole("button", { name: "Delete 40 nodes" })).toBeVisible();
    });
  },
};

/** A Studio reopened read-only offers nothing to act with, so it draws nothing. */
export const NothingOffered: Story = {
  args: { acts: [], onAct: fn() },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button")).toBeNull();
  },
};
