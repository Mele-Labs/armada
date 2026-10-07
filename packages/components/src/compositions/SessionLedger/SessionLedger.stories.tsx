import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { SessionLedger, type LedgerEntry } from "./SessionLedger";

/** What a Session has accumulated, a group per kind. Each row opens its own surface. */
const meta: Meta<typeof SessionLedger> = {
  title: "Compositions/Session ledger",
  component: SessionLedger,
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "calc(var(--space-12) * 8)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionLedger>;

const open = fn();
const ENTRIES: LedgerEntry[] = [
  { key: "s3", kind: "slot", name: "Worktree slot 3", text: "3", onOpen: open },
  { key: "b", kind: "branch", name: "Branch fix/flaky-store", text: "fix/flaky-store", onOpen: open },
  { key: "p", kind: "pull_request", name: "Pull request #1843, checks failed", text: "#1843 Pin the store clock", mark: { glyph: "failed", said: "Checks failed: store: 2 failed" }, onOpen: open },
  { key: "j", kind: "job", name: "Job 52", text: "52 Pin the store clock", slot: 4, onOpen: open },
  { key: "st", kind: "studio", name: "Studio Flaky store shapes", text: "Flaky store shapes", onOpen: open },
  { key: "k", kind: "sketch", name: "Sketch Store clock", text: "Store clock", onOpen: open },
  { key: "a", kind: "subagent", name: "Subagent Read the CI history, running", text: "Read the CI history", mark: { glyph: "running", said: "Running" }, onOpen: open },
];

/** Nothing attached: every section is there, dim, over an empty well. */
export const Blank: Story = {
  args: { entries: [] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("listitem")).toBeNull();
    await expect(canvas.getByRole("heading", { name: "Worktree Slot" })).toBeInTheDocument();
    await expect(canvas.getByRole("heading", { name: "Sketches" })).toBeInTheDocument();
  },
};

export const Accumulated: Story = {
  args: { entries: ENTRIES },
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open Job 52" }));
    await expect(open).toHaveBeenCalled();
  },
};
