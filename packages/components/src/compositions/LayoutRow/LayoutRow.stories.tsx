import type { Meta, StoryObj } from "@storybook/react-vite";
import { GitMerge } from "lucide-react";

import { LayoutRow } from "./LayoutRow";

const meta: Meta<typeof LayoutRow> = {
  title: "Compositions/Layout row",
  component: LayoutRow,
  args: { label: "Merge line", icon: GitMerge, visible: true, hideable: true, onVisible: () => undefined, onUp: () => undefined, onDown: () => undefined },
};
export default meta;

type Story = StoryObj<typeof LayoutRow>;

export const Shown: Story = {};
export const Hidden: Story = { args: { visible: false } };
export const HiddenByAMod: Story = { args: { visible: false, hiddenBy: "Tidy" } };
export const CannotBeHidden: Story = { args: { label: "Command Central", hideable: false, onUp: undefined } };
