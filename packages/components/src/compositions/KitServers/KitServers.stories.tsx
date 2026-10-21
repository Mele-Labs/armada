import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { KitServerAddForm } from "./KitServers";

const meta: Meta<typeof KitServerAddForm> = {
  title: "Compositions/Kit servers",
  component: KitServerAddForm,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
  args: { onAdd: fn() },
};
export default meta;

type Story = StoryObj<typeof KitServerAddForm>;

export const Empty: Story = {};

/** A name and a program are what adding takes, and adding turns nothing on. */
export const AddingAProgram: Story = {
  play: async ({ args, canvasElement }) => {
    const form = within(canvasElement);
    await userEvent.type(form.getByLabelText("Name"), "tracker");
    await userEvent.type(form.getByLabelText("Program"), "npx -y @scope/server");
    await userEvent.click(form.getByRole("button", { name: "Add" }));
    await expect(args.onAdd).toHaveBeenCalledWith({ name: "tracker", kind: "stdio", address: "npx -y @scope/server" });
  },
};
