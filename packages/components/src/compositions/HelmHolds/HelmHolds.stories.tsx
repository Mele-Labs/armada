import type { Meta, StoryObj } from "@storybook/react-vite";
import { HelmHolds } from "./HelmHolds";

const meta: Meta<typeof HelmHolds> = {
  title: "Compositions/Helm holds",
  component: HelmHolds,
};
export default meta;

type Story = StoryObj<typeof HelmHolds>;

const open = () => undefined;

/** Helm has written once: the slot it leased and the branch it cut, under the dock's head. */
export const ASlotAndABranch: Story = {
  args: {
    entries: [
      { key: "slot", kind: "slot", name: "Slot 14", text: "slot-14", onOpen: open },
      { key: "branch", kind: "branch", name: "Branch helm/route-thoughts", text: "helm/route-thoughts", onOpen: open },
    ],
  },
};

/** A subagent Helm runs, its turn as the mark at the chip's end. */
export const ASubagentWorking: Story = {
  args: {
    entries: [
      { key: "slot", kind: "slot", name: "Slot 14", text: "slot-14", onOpen: open },
      { key: "agent", kind: "subagent", name: "Subagent Read the merge line", text: "Read the merge line", mark: { glyph: "running", said: "Working" }, onOpen: open },
    ],
  },
};

/** Nothing held draws nothing. */
export const NothingHeld: Story = {
  args: { entries: [] },
};
