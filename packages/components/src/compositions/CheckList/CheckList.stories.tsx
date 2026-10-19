import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { Tabs } from "../../primitives/Tabs/Tabs";
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
  { id: "a", name: "test", status: "running", says: "running", startedAt: "2026-09-12T14:19:40Z" },
  { id: "b", name: "typecheck", status: "waiting", says: "waiting" },
  { id: "c", name: "build", status: "passed", says: "passed", startedAt: "2026-09-12T14:19:18Z", duration: "41.3s" },
  { id: "d", name: "format", status: "failed", says: "failed", startedAt: "2026-09-12T14:11:30Z", duration: "9.4s" },
  { id: "e", name: "storybook", status: "stopped", says: "stopped", startedAt: "2026-09-12T14:09:02Z", duration: "3.0s" },
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

/** The host's filters in the band over the glass, as the Checks page draws them. */
const filters = (
  <Tabs
    items={["All", "Active", "Waiting", "Completed", "Passed", "Failed", "Skipped"].map((label) => ({ id: label, label }))}
    value="All"
    onChange={fn()}
  />
);

export const WithFilters: Story = { args: { rows, openRow: "c", onOpenRow: fn(), controls: filters } };

/** A filter that holds nothing: the band stays, so the filter can be changed back. */
export const FilteredToNothing: Story = { args: { rows: [], onOpenRow: fn(), controls: filters } };

export const Details: Story = {
  render: () => (
    <CheckDetails
      details={[
        { label: "Command", value: "cargo build --workspace --locked", mono: true },
        { label: "Started", at: "2026-09-12T14:19:18Z" },
        { label: "Took", value: "41.3s" },
        { label: "Exit", value: "0, expected 0" },
      ]}
    />
  ),
};
