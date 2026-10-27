import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { PaletteRows, type PaletteRowView } from "./PaletteRows";

const ROWS: PaletteRowView[] = [
  { id: "session:cache-key", name: "Cache key", kind: "Job", alias: "", hotkey: "", pinned: true, keyable: true },
  { id: "pull:armada:1899", name: "Cache the plan windows for a minute", kind: "Pull request", alias: "cache", hotkey: "", pinned: false, keyable: true },
  { id: "page:reviews", name: "Reviews", kind: "Page", alias: "rev", hotkey: "⌃⌘R", pinned: false, keyable: true },
];

const meta: Meta<typeof PaletteRows> = {
  title: "Compositions/Palette rows",
  component: PaletteRows,
  args: { rows: ROWS, onAlias: fn(() => undefined), onHotkey: fn(() => undefined), onPin: fn() },
};
export default meta;

type Story = StoryObj<typeof PaletteRows>;

/** Settings' Command palette table: every row with an alias, a hotkey or a pin, each editable in place. */
export const AsKept: Story = {
  play: async ({ canvasElement }) => {
    const table = within(canvasElement);
    await expect(table.getByText("Reviews")).toBeVisible();
    await expect(table.getByDisplayValue("rev")).toBeVisible();
  },
};

/** A row that runs nothing by itself takes an alias and a pin, and no key. */
export const NotKeyable: Story = {
  args: { rows: [{ id: "setting:spacing", name: "Spacing", kind: "Setting", alias: "", hotkey: "", pinned: false, keyable: false }] },
};
