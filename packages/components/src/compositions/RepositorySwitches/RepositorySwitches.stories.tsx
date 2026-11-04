import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { RepositorySwitches } from "./RepositorySwitches";

const meta: Meta<typeof RepositorySwitches> = {
  title: "Compositions/Repository switches",
  component: RepositorySwitches,
  args: {
    title: "Connectors",
    help: "Off, Jobs in that repository start without the connectors on your account.",
    rows: [
      {
        id: "armada",
        name: "armada",
        on: false,
      },
      {
        id: "storefront",
        name: "storefront",
        on: true,
      },
    ],
    switchLabel: (name: string) => `Connectors in ${name}`,
    onSwitch: fn(),
  },
  decorators: [
    (Story) => (
      <div
        style={{
          padding: "var(--space-6)",
          background: "var(--surface-canvas)",
          maxWidth: "var(--w-dialog-wide)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof RepositorySwitches>;

/** One repository off and one on, each switch in the same column. */
export const OneOffOneOn: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("switch", { name: "Connectors in armada" })).not.toBeChecked();
    await expect(canvas.getByRole("switch", { name: "Connectors in storefront" })).toBeChecked();
  },
};
