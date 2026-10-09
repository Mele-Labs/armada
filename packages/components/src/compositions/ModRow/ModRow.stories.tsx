import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { Card, CardContent } from "../../primitives/Card/Card";
import { ModRow } from "./ModRow";

const meta: Meta<typeof ModRow> = {
  title: "Compositions/Mod row",
  component: ModRow,
  args: { title: "Dusk", enabled: true, onEnabled: fn(), onPromote: fn() },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-dialog-wide)" }}>
        <Card>
          <CardContent>
            <Story />
          </CardContent>
        </Card>
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ModRow>;

/** On and valid, which is where a new mod starts. */
export const OnAndValid: Story = {
  play: async ({ canvasElement }) => {
    const row = within(canvasElement).getByRole("group", { name: "Dusk" });
    await expect(within(row).getByRole("img", { name: "Theme" })).toBeVisible();
    await expect(within(row).getByRole("switch", { name: "Dusk" })).toBeChecked();
  },
};

export const SwitchedOff: Story = { args: { enabled: false } };

/** Fleet's reason, read exactly. Promote is off because a mod that fails its checks has nothing to promote. */
export const Invalid: Story = {
  args: { detail: "line 3: `--color-x` is not a token", promoteDisabled: true },
};

/** The branch it was put on, and nothing else. Promote is spent. */
export const OnABranch: Story = {
  args: { detail: "armada/mod-dusk-1791500000000", promoteDisabled: true },
};

/** The two presses reach their callbacks: the switch with the new value, Promote once. */
export const Presses: Story = {
  play: async ({ canvasElement, args }) => {
    const row = within(canvasElement).getByRole("group", { name: "Dusk" });
    await userEvent.click(within(row).getByRole("switch", { name: "Dusk" }));
    await expect(args.onEnabled).toHaveBeenCalledWith(false);
    await userEvent.click(within(row).getByRole("button", { name: "Promote Dusk" }));
    await expect(args.onPromote).toHaveBeenCalledTimes(1);
  },
};
