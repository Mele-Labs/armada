import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { CheckDetails, CheckList, type CheckListRow } from "./CheckList";

/**
 * One story per thing the list has to say: every status a row can hold, and the
 * facts of one Check. The state is a mark named by its tooltip, never a word.
 */
const meta: Meta<typeof CheckList> = {
  title: "Compositions/Check list",
  component: CheckList,
};
export default meta;

type Story = StoryObj<typeof CheckList>;

const rows: CheckListRow[] = [
  { id: "a", name: "test", status: "running", says: "running", started: "14:19:40", startedExact: "12 Sep 2026, 14:19:40" },
  { id: "b", name: "typecheck", status: "waiting", says: "waiting" },
  { id: "c", name: "build", status: "passed", says: "passed", started: "14:19:18", duration: "41.3s" },
  { id: "d", name: "format", status: "failed", says: "failed", started: "14:11:30", duration: "9.4s" },
  { id: "e", name: "storybook", status: "stopped", says: "stopped", started: "14:09:02", duration: "3.0s" },
];

export const EveryStatus: Story = {
  args: { rows, openRow: "c", onOpenRow: fn() },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByText("running").length).toBeGreaterThan(0);
    await expect(canvas.getAllByText("waiting").length).toBeGreaterThan(0);
    await expect(canvas.getByRole("button", { name: "build" })).toHaveAttribute("aria-expanded", "true");
  },
};

export const NoRows: Story = { args: { rows: [] } };

export const Details: Story = {
  render: () => (
    <CheckDetails
      details={[
        { label: "Command", value: "cargo build --workspace --locked", mono: true },
        { label: "Started", value: "12 Sep 2026, 14:19:18" },
        { label: "Took", value: "41.3s" },
        { label: "Exit", value: "0, expected 0" },
      ]}
    />
  ),
};
