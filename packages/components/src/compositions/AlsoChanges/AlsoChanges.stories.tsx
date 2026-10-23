import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { AlsoChanges } from "./AlsoChanges";

const meta: Meta<typeof AlsoChanges> = {
  title: "Compositions/Also changes",
  component: AlsoChanges,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-dialog-wide)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof AlsoChanges>;

/** Both kinds of mod a teaching can make, each named and opening in Mods. */
export const WordsAndFleet: Story = {
  args: {
    lines: [
      { part: "Words", text: "Notices say “behind main” for “out of date”", mod: { name: "Plain notices", onOpen: fn() } },
      { part: "Fleet", text: "Holds a Job at a “checks failed” notice", mod: { name: "Hold on failed checks", onOpen: fn() } },
    ],
  },
  play: async ({ canvasElement }) => {
    const group = within(canvasElement).getByRole("group", { name: "Also changes" });
    await expect(within(group).getAllByRole("listitem")).toHaveLength(2);
    await expect(within(group).getByRole("button", { name: "Open Plain notices in Mods" })).toBeVisible();
  },
};

export const OneLine: Story = {
  args: { lines: [{ part: "Words", text: "Notices say “behind main” for “out of date”", mod: { name: "Plain notices", onOpen: fn() } }] },
};

/** A change whose mod is not named has nothing to open. */
export const WithoutAMod: Story = {
  args: { lines: [{ part: "Fleet", text: "Holds a Job at a “checks failed” notice" }] },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("button")).toBeNull();
  },
};

/** Nothing else changed, so nothing is drawn. */
export const None: Story = {
  args: { lines: [] },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("group", { name: "Also changes" })).toBeNull();
  },
};

export const OpensItsMod: Story = {
  args: { lines: [{ part: "Words", text: "Notices say “behind main” for “out of date”", mod: { name: "Plain notices", onOpen: fn() } }] },
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Open Plain notices in Mods" }));
    await expect(args.lines[0]?.mod?.onOpen).toHaveBeenCalledTimes(1);
  },
};
