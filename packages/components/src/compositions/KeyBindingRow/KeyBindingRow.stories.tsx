import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { Power } from "lucide-react";

import { parseSlots } from "../../keymap";
import { KeyBindingRow } from "./KeyBindingRow";

const meta: Meta<typeof KeyBindingRow> = {
  title: "Compositions/Key binding row",
  component: KeyBindingRow,
  args: { verb: "Kill", icon: Power, slots: [{ slot: parseSlots("x")[0]! }], listening: null, onListen: fn(), onBind: fn() },
};
export default meta;

type Story = StoryObj<typeof KeyBindingRow>;

export const AsShipped: Story = {};

export const Rebound: Story = { args: { slots: [{ slot: parseSlots("q")[0]! }], onReset: fn() } };

export const TwoKeys: Story = {
  args: { verb: "Back and forward", icon: null, slots: parseSlots("⌘[ ⌘]").map((slot, at) => ({ slot, name: at === 0 ? "Back" : "Forward" })) },
};

export const ARowOfDigits: Story = { args: { verb: "Bridge surfaces", icon: null, slots: parseSlots("⌘1–⌘9").map((slot) => ({ slot })) } };

export const Unbound: Story = { args: { slots: [{ slot: { kind: "chord", chord: null } }], onReset: fn() } };

export const SharesAKey: Story = { args: { slots: [{ slot: parseSlots("c")[0]!, conflicts: ["Copy debug info"] }], onReset: fn() } };

/** Pressing a key control listens; the next key pressed is the binding, spelled as the registry spells one. */
export const Listening: Story = {
  args: { listening: 0 },
  play: async ({ args, canvasElement }) => {
    await expect(within(canvasElement).getByText("Press a key")).toBeVisible();
    await userEvent.keyboard("{Shift>}q{/Shift}");
    await expect(args.onBind).toHaveBeenCalledWith(0, "Q");
    await expect(args.onListen).toHaveBeenCalledWith(null);
  },
};
