import { useState } from "react";
import type { ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import SHIPPED from "@armada/tokens/themes/index.json";
import { ThemePicker } from "./ThemePicker";
import type { ThemeGroup } from "./ThemePicker";

/**
 * The Theme field in Settings: Dark, Light and the themes mods ship, as one list narrowed by typing.
 *
 * **Every state here is one a person meets.** The chosen theme, the list open on it, and the list
 * narrowed to what was typed with the groups that no longer match gone.
 */
/** A shipped theme's own strip, as `index.json` carries it. */
const swatchOf = (id: string): string[] => SHIPPED.find((one) => one.id === id)?.swatch ?? [];
/** The tokens in force on the page, which is how Dark is previewed: its tokens are the page's own. */
const DARK_NOW = ["--bg-base", "--bg-raised", "--fg-default", "--accent", "--status-completed-success", "--status-completed-failed", "--status-awaiting-review", "--status-running"].map((name) => `var(${name})`);

const GROUPS: ThemeGroup[] = [
  { label: "Dark", choices: [{ id: "dark", title: "Dark", swatch: DARK_NOW }, { id: "catalogue:nord", title: "Nord", swatch: swatchOf("catalogue:nord") }, { id: "catalogue:tokyonight", title: "Tokyo Night", swatch: swatchOf("catalogue:tokyonight") }] },
  { label: "Light", choices: [{ id: "light", title: "Light" }, { id: "catalogue:nord-light", title: "Nord Light", swatch: swatchOf("catalogue:nord-light") }] },
  // A mod whose colours have not been checked yet has none to show.
  { label: "From mods", choices: [{ id: "dusk", title: "Dusk" }] },
];

const meta: Meta<typeof ThemePicker> = {
  title: "Compositions/Theme picker",
  component: ThemePicker,
  args: { label: "Theme", value: "dark", groups: GROUPS, onValue: fn() },
};
export default meta;

type Story = StoryObj<typeof ThemePicker>;

/** A field whose value feeds back, which is what a picked theme has to survive. */
function Picking(props: ComponentProps<typeof ThemePicker>) {
  const [value, setValue] = useState(props.value);
  return (
    <ThemePicker
      {...props}
      value={value}
      onValue={(id) => {
        setValue(id);
        props.onValue(id);
      }}
    />
  );
}

/** Shut: the field says the chosen theme, with its strip, and nothing else. */
export const Chosen: Story = {
  render: (args) => <Picking {...args} value="catalogue:nord" />,
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("combobox", { name: "Theme" })).toHaveValue("Nord");
    await expect(within(canvasElement).queryByRole("listbox")).toBeNull();
    await expect(strip(canvasElement)).toEqual(swatchOf("catalogue:nord").map(rgb));
  },
};

/** The colours a strip paints, in order, as the browser resolves them. */
const strip = (root: Element, at = 0): string[] =>
  Array.from(root.querySelectorAll(".armada-themepick__swatch")[at]?.children ?? []).map((cell) => getComputedStyle(cell).backgroundColor);
/** A hex colour as `getComputedStyle` spells it. */
const rgb = (hex: string): string => {
  const n = parseInt(hex.slice(1), 16);
  return `rgb(${n >> 16}, ${(n >> 8) & 255}, ${n & 255})`;
};

/** A theme whose colours are not known draws the same strip in neutral, so no row moves when they arrive. */
export const Unknown: Story = {
  render: (args) => <Picking {...args} value="dusk" />,
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("combobox", { name: "Theme" }));
    const cells = strip(canvasElement);
    await expect(cells).toHaveLength(8);
    await expect(new Set(cells).size).toBe(1);
  },
};

/** Open on the chosen theme, in its three groups. */
export const Open: Story = {
  render: (args) => <Picking {...args} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("combobox", { name: "Theme" }));
    await expect(canvas.getByRole("group", { name: "Dark" })).toBeVisible();
    await expect(canvas.getByRole("group", { name: "Light" })).toBeVisible();
    await expect(canvas.getByRole("group", { name: "From mods" })).toBeVisible();
    await expect(canvas.getByRole("option", { name: "Dark" })).toHaveAttribute("aria-selected", "true");
    // Each row carries its own theme's colours, not a neighbour's: the second strip is Nord's.
    const options = canvasElement.querySelectorAll('[role="option"]');
    const nord = Array.from(options[1]?.querySelectorAll(".armada-themepick__cell") ?? []).map((cell) => getComputedStyle(cell).backgroundColor);
    await expect(nord).toEqual(swatchOf("catalogue:nord").map(rgb));
  },
};

/** Typing narrows to the titles that match, and a group with none is not drawn. */
export const Narrowed: Story = {
  render: (args) => <Picking {...args} />,
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.type(canvas.getByRole("combobox", { name: "Theme" }), "nord");
    await expect(canvas.getAllByRole("option").map((one) => one.textContent)).toEqual(["Nord", "Nord Light"]);
    await expect(canvas.queryByRole("group", { name: "From mods" })).toBeNull();
    await userEvent.keyboard("{ArrowDown}{Enter}");
    await expect(args.onValue).toHaveBeenCalledWith("catalogue:nord-light");
    await expect(canvas.getByRole("combobox", { name: "Theme" })).toHaveValue("Nord Light");
  },
};
