import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SettingsIndex } from "./SettingsIndex";

const SECTIONS = [
  { id: "fleet", label: "Fleet" },
  { id: "machine", label: "This machine" },
  { id: "theme", label: "Theme" },
  { id: "keyboard", label: "Keyboard shortcuts" },
];

const meta: Meta<typeof SettingsIndex> = {
  title: "Compositions/Settings index",
  component: SettingsIndex,
  args: { sections: SECTIONS, current: "theme", query: "", onSection: fn(), onQuery: fn(), children: <p>The chosen category.</p> },
};
export default meta;

type Story = StoryObj<typeof SettingsIndex>;

/** One category chosen, the rest down the side. Choosing another says which. */
export const OneCategory: Story = {
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("tab", { name: "Theme" })).toHaveAttribute("aria-selected", "true");
    await userEvent.click(canvas.getByRole("tab", { name: "Keyboard shortcuts" }));
    await expect(args.onSection).toHaveBeenCalledWith("keyboard");
  },
};

/** A search on: only the categories holding a match, each with how many. */
export const Searching: Story = {
  args: {
    query: "key",
    sections: SECTIONS.map((one) => ({ ...one, matches: one.id === "keyboard" ? 12 : one.id === "machine" ? 1 : 0 })),
    children: <p>Every match.</p>,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("tab", { name: /^Keyboard shortcuts/ })).toHaveTextContent("12");
    await expect(canvas.queryByRole("tab", { name: /Theme/ })).toBeNull();
  },
};

export const NothingMatches: Story = {
  args: { query: "zebra", sections: SECTIONS.map((one) => ({ ...one, matches: 0 })), children: null },
};
