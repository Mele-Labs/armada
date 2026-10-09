import { useState } from "react";
import type { ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ThemePicker } from "./ThemePicker";
import type { ThemeGroup } from "./ThemePicker";

/**
 * The Theme field in Settings: Dark, Light and the themes mods ship, as one list narrowed by typing.
 *
 * **Every state here is one a person meets.** The chosen theme, the list open on it, and the list
 * narrowed to what was typed with the groups that no longer match gone.
 */
const GROUPS: ThemeGroup[] = [
  { label: "Dark", choices: [{ id: "dark", title: "Dark" }, { id: "catalogue:nord", title: "Nord" }, { id: "catalogue:dracula", title: "Dracula" }] },
  { label: "Light", choices: [{ id: "light", title: "Light" }, { id: "catalogue:nord-light", title: "Nord Light" }] },
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

/** Shut: the field says the chosen theme and nothing else. */
export const Chosen: Story = {
  render: (args) => <Picking {...args} />,
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("combobox", { name: "Theme" })).toHaveValue("Dark");
    await expect(within(canvasElement).queryByRole("listbox")).toBeNull();
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
