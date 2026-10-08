import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

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
const shown = fn();
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
  { key: "ar4", kind: "artifact", artifact: "image", name: "Looked at clock-trace.png", text: "clock-trace.png", onOpen: open },
  { key: "ar3", kind: "artifact", artifact: "doc", name: "Doc Flaky store write-up", text: "Flaky store write-up", onOpen: open },
  { key: "ar4", kind: "artifact", artifact: "window", name: "Shown in a window Store clock findings", text: "Store clock findings", onOpen: shown },
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

/** A page, a file, a picture looked at, a Doc and a window, each with its own glyph named by a tooltip. */
export const Artifacts: Story = {
  args: { entries: ENTRIES.filter((one) => one.kind === "artifact") },
  play: async ({ canvas }) => {
    localStorage.clear();
    await expect(canvas.getByRole("heading", { name: "Artifacts" })).toBeInTheDocument();
    // Pictures start out of the list.
    await expect(canvas.queryByRole("img", { name: "Looked at" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Pictures" }));
    await expect(canvas.getByRole("img", { name: "Published page" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "File written" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Doc" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Looked at" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Open File written clock-trace.png" }));
    await expect(open).toHaveBeenCalled();
    // A page shown in a window: its glyph names it, and the press reopens the window.
    await expect(canvas.getByRole("img", { name: "Shown in a window" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Open Shown in a window Store clock findings" }));
    await expect(shown).toHaveBeenCalledTimes(1);
    const before = open.mock.calls.length;
    await userEvent.click(canvas.getByRole("button", { name: "Open Looked at clock-trace.png" }));
    await expect(open).toHaveBeenCalledTimes(before + 1);
    localStorage.clear();
  },
};

const MANY: LedgerEntry[] = [
  ...Array.from({ length: 8 }, (_, i): LedgerEntry => ({ key: `f${i}`, kind: "artifact", artifact: i === 0 ? "window" : "file", name: `${i === 0 ? "Shown in a window" : "File written"} n${i}`, text: `n${i}`, onOpen: open })),
  ...Array.from({ length: 3 }, (_, i): LedgerEntry => ({ key: `i${i}`, kind: "artifact", artifact: "image", name: `Looked at p${i}.png`, text: `p${i}.png`, onOpen: open })),
];

/** Pictures start hidden, a window and files show, only the newest five do until More, and Less folds back. */
export const ArtifactsFiltered: Story = {
  args: { entries: MANY },
  play: async ({ canvas }) => {
    localStorage.clear();
    const artifacts = within(canvas.getByRole("region", { name: "Artifacts" }));
    await expect(artifacts.getByRole("button", { name: "Pictures" })).toHaveAttribute("aria-pressed", "false");
    await expect(artifacts.queryByRole("button", { name: /Looked at/ })).toBeNull();
    await expect(artifacts.getByRole("button", { name: "Open Shown in a window n0" })).toBeInTheDocument();
    await expect(artifacts.getAllByRole("listitem")).toHaveLength(5);
    await userEvent.click(artifacts.getByRole("button", { name: "More" }));
    await expect(artifacts.getAllByRole("listitem")).toHaveLength(8);
    await userEvent.click(artifacts.getByRole("button", { name: "Less" }));
    await expect(artifacts.getAllByRole("listitem")).toHaveLength(5);
    await userEvent.click(artifacts.getByRole("button", { name: "Pictures" }));
    await expect(artifacts.queryByRole("button", { name: "Open Looked at p0.png" })).toBeNull();
    await userEvent.click(artifacts.getByRole("button", { name: "More" }));
    await expect(artifacts.getByRole("button", { name: "Open Looked at p0.png" })).toBeInTheDocument();
    // The filter hiding every row leaves the head and toggles.
    for (const name of ["Windows", "Files", "Pictures"]) await userEvent.click(artifacts.getByRole("button", { name }));
    await expect(artifacts.queryByRole("listitem")).toBeNull();
    await expect(artifacts.getByRole("button", { name: "Files" })).toBeInTheDocument();
    await expect(artifacts.queryByRole("button", { name: "Docs" })).toBeNull();
    localStorage.clear();
  },
};

export const Accumulated: Story = {
  args: { entries: ENTRIES },
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open Job 52" }));
    await expect(open).toHaveBeenCalled();
  },
};

const FINISHED: LedgerEntry[] = [
  { key: "pm1", kind: "pull_request", name: "Pull request #1801, checks passed", text: "#1801 Merged one", finished: true, onOpen: open },
  { key: "po", kind: "pull_request", name: "Pull request #1843, checks passed", text: "#1843 Open one", onOpen: open },
  { key: "ad", kind: "subagent", name: "Subagent Read the logs, done", text: "Read the logs", finished: true, onOpen: open },
];

/** A section with finished rows opens on Open, All adds them, and each section remembers its own choice. */
export const OpenAndAll: Story = {
  args: { entries: FINISHED },
  play: async ({ canvas }) => {
    localStorage.clear();
    await expect(canvas.queryByRole("button", { name: /#1801/ })).toBeNull();
    await expect(canvas.getByRole("button", { name: /#1843/ })).toBeInTheDocument();
    const pulls = canvas.getByRole("region", { name: "Pull requests" });
    await userEvent.click(within(pulls).getByRole("radio", { name: "All" }));
    await expect(canvas.getByRole("button", { name: /#1801/ })).toBeInTheDocument();
    // Subagents kept its own choice: its one finished row is still out.
    await expect(canvas.queryByRole("button", { name: /Read the logs/ })).toBeNull();
    // The header and toggle stay where the Open view is empty.
    const agents = canvas.getByRole("region", { name: "Subagents" });
    await expect(within(agents).getByRole("radio", { name: "Open" })).toBeChecked();
    await expect(within(agents).queryByRole("listitem")).toBeNull();
    await userEvent.click(within(agents).getByRole("radio", { name: "All" }));
    await expect(canvas.getByRole("button", { name: /Read the logs/ })).toBeInTheDocument();
    localStorage.clear();
  },
};
