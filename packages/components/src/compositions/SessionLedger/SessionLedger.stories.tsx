import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { SessionLedger, type LedgerEntry } from "./SessionLedger";

/** What a Session has accumulated, a group per kind and only the groups that hold a row. Each row opens its own surface. */
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
  { key: "ar1", kind: "artifact", artifact: "page", name: "Published page Store clock findings", text: "Store clock findings", onOpen: open },
  { key: "ar2", kind: "artifact", artifact: "file", name: "File written clock-trace.png", text: "clock-trace.png", onOpen: open },
  { key: "ar3", kind: "artifact", artifact: "doc", name: "Doc Flaky store write-up", text: "Flaky store write-up", onOpen: open },
];

/** Nothing attached: no section is drawn, only the small picture, and no sentence. */
export const Blank: Story = {
  args: { entries: [] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("listitem")).toBeNull();
    await expect(canvas.queryByRole("heading")).toBeNull();
    await expect(canvas.getByRole("img", { name: "Nothing attached" })).toBeInTheDocument();
    await expect(canvas.getByRole("region", { name: "Attachments" }).textContent).toBe("");
  },
};

/** Only the sections that hold a row are drawn, and the picture is gone. */
export const Some: Story = {
  args: { entries: ENTRIES.filter((one) => one.kind === "branch" || one.kind === "job") },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("heading", { name: "Branches" })).toBeInTheDocument();
    await expect(canvas.getByRole("heading", { name: "Jobs" })).toBeInTheDocument();
    await expect(canvas.queryByRole("heading", { name: "Worktree Slot" })).toBeNull();
    await expect(canvas.queryByRole("heading", { name: "Artifacts" })).toBeNull();
    await expect(canvas.queryByRole("img", { name: "Nothing attached" })).toBeNull();
  },
};

/** A page, a file and a Doc, each with its own glyph named by a tooltip. */
export const Artifacts: Story = {
  args: { entries: ENTRIES.filter((one) => one.kind === "artifact") },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("heading", { name: "Artifacts" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Published page" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "File written" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Doc" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Open File written clock-trace.png" }));
    await expect(open).toHaveBeenCalled();
  },
};

export const Accumulated: Story = {
  args: { entries: ENTRIES },
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open Job 52" }));
    await expect(open).toHaveBeenCalled();
  },
};
