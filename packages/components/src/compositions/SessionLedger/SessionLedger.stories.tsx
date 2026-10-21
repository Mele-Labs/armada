import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, userEvent, waitFor, within } from "storybook/test";

import { BellRing } from "lucide-react";

import { LedgerPreview, LedgerRows, SessionLedger, type LedgerEntry } from "./SessionLedger";

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
  { key: "s7", kind: "slot", name: "Worktree slot 7", text: "7", onOpen: open },
  { key: "b", kind: "branch", name: "Branch fix/flaky-store", text: "fix/flaky-store", slot: 3, onOpen: open },
  { key: "p", kind: "pull_request", name: "Pull request #1843, checks failed", text: "#1843 Pin the store clock", mark: { glyph: "failed", said: "Checks failed: store: 2 failed" }, onOpen: open },
  { key: "st", kind: "studio", name: "Studio Flaky store shapes", text: "Flaky store shapes", onOpen: open },
  { key: "k", kind: "sketch", name: "Sketch Store clock", text: "Store clock", onOpen: open },
  { key: "a", kind: "subagent", name: "Subagent Read the CI history, running", text: "Read the CI history", mark: { glyph: "running", said: "Running" }, onOpen: open },
  { key: "bg1", kind: "background", background: "command", name: "Background command Serve the Bridge mock from slot 6, running", text: "Serve the Bridge mock from slot 6", mark: { glyph: "running", said: "Running" }, onOpen: open },
  { key: "bg2", kind: "background", background: "monitor", name: "Monitor Watch the CI run, running", text: "Watch the CI run", mark: { glyph: "running", said: "Running" }, onOpen: open },
  { key: "ar1", kind: "artifact", artifact: "page", name: "Published page Store clock findings", text: "Store clock findings", onOpen: open },
  { key: "ar2", kind: "artifact", artifact: "file", name: "File written clock-trace.png", text: "clock-trace.png", onOpen: open },
  { key: "ar4", kind: "artifact", artifact: "image", name: "Looked at clock-trace.png", text: "clock-trace.png", onOpen: open },
  { key: "ar3", kind: "artifact", artifact: "doc", name: "Doc Flaky store write-up", text: "Flaky store write-up", onOpen: open },
  { key: "ar4", kind: "artifact", artifact: "window", name: "Shown in a window Store clock findings", text: "Store clock findings", onOpen: shown },
];

/** The ledger draws no waiting rows: the Needs you panel on the composer does, with these. */
const inTheBand: Story["render"] = (args) => <LedgerRows kind="waiting" Glyph={BellRing} rows={args.entries} />;

const picked = fn();
const WAITING: LedgerEntry[] = [
  { key: "w1", kind: "waiting", act: "walk", name: "Look at the findings page", text: "Look at the findings page", onOpen: shown },
  { key: "w2", kind: "waiting", act: "approve_pr", name: "Approve #1843", text: "Approve #1843", onOpen: open },
  { key: "w3", kind: "waiting", act: "run", name: "cargo test -p store", text: <code>cargo test -p store</code>, onOpen: open },
  {
    key: "w4",
    kind: "waiting",
    act: "answer",
    name: "Which clock?",
    text: "Which clock?",
    options: [
      { label: "Fake", onPick: picked },
      { label: "Frozen", onPick: picked },
    ],
    onOpen: open,
  },
];

/** What waits on the person, as the Needs you band draws it: each item names its act by glyph, and a question's choices are a deck. The ledger itself draws none. */
export const WaitingOnYou: Story = {
  args: { entries: WAITING },
  render: inTheBand,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Opens the walk" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Opens the pull request" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Copies the command" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Open Look at the findings page" }));
    await expect(shown).toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("radio", { name: "Frozen" }));
    await userEvent.click(canvas.getByRole("button", { name: "Answer" }));
    await expect(picked).toHaveBeenCalledTimes(1);
  },
};

/** A waiting item handed to the ledger is not drawn: the Needs you panel holds it, and a ledger of nothing else is the empty picture. */
export const NoWaitingSection: Story = {
  args: { entries: [WAITING[1]!] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("heading", { name: "Waiting on you" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Open Approve #1843" })).toBeNull();
    await expect(canvas.getByRole("img", { name: "Nothing attached" })).toBeInTheDocument();
    await expect(canvas.queryByRole("group", { name: "Narrow the ledger" })).toBeNull();
  },
};

let delivered: () => void = () => undefined;
const sent = fn(() => new Promise<void>((done) => (delivered = done)));
/** While Fleet delivers an answer, Answer is disabled, so a second press cannot send it again. */
export const AnswerSentOnce: Story = {
  render: inTheBand,
  args: { entries: [{ ...WAITING[3]!, key: "w6", options: [{ label: "Fake", onPick: sent }, { label: "Frozen", onPick: sent }] }] },
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("radio", { name: "Frozen" }));
    const answer = canvas.getByRole("button", { name: "Answer" });
    await userEvent.click(answer);
    await expect(answer).toBeDisabled();
    await expect(sent).toHaveBeenCalledTimes(1);
    delivered();
    await waitFor(() => expect(answer).toBeEnabled());
  },
};

const replied = fn();
/** An item with no act and a reply: its glyph is Reply, a press opens a box under the row, and a send hands over the line. */
export const WaitingReply: Story = {
  render: inTheBand,
  args: {
    entries: [
      { key: "w5", kind: "waiting", name: "Keep the old fixture or drop it?", text: "Keep the old fixture or drop it?", onReply: replied, onOpen: open },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Reply" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Open Keep the old fixture or drop it?" }));
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
    await userEvent.type(canvas.getByRole("textbox", { name: "Reply to Keep the old fixture or drop it?" }), "Drop it{Enter}");
    await expect(replied).toHaveBeenCalledWith("Drop it");
    await expect(canvas.queryByRole("textbox")).not.toBeInTheDocument();
  },
};

const removed = fn();
/** An item the agent stated has Remove: a press opens a one-line reason box, Enter removes with the words or without, and Esc removes nothing. */
export const WaitingRemove: Story = {
  render: inTheBand,
  args: {
    entries: [{ key: "w6", kind: "waiting", name: "Pick a name for the cache table", text: "Pick a name for the cache table", onRemove: removed, onOpen: open }],
  },
  play: async ({ canvas }) => {
    removed.mockClear();
    const act = canvas.getByRole("button", { name: "Remove Pick a name for the cache table" });
    const reason = () => canvas.queryByRole("textbox", { name: "Reason for removing Pick a name for the cache table" });
    await userEvent.click(act);
    await expect(reason()).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    await expect(reason()).not.toBeInTheDocument();
    await expect(removed).not.toHaveBeenCalled();
    await userEvent.click(act);
    await userEvent.type(reason()!, "The cache tests still read it.{Enter}");
    await expect(removed).toHaveBeenLastCalledWith("The cache tests still read it.");
    await expect(reason()).not.toBeInTheDocument();
    await userEvent.click(act);
    await userEvent.keyboard("{Enter}");
    await expect(removed).toHaveBeenLastCalledWith("");
    await expect(removed).toHaveBeenCalledTimes(2);
  },
};

const releasing = fn();

/** Release on a held row, shown on hover or focus; a row given back stays drawn with its mark, and a row Fleet kept says why. */
export const ReleaseRow: Story = {
  args: {
    entries: [
      { key: "s4", kind: "slot", name: "Worktree slot 4", text: "4", release: { name: "slot-4", onPress: releasing }, onOpen: open },
      { key: "s7", kind: "slot", name: "Worktree slot 7", text: "7", released: { kept: "Another Session holds it." }, release: { name: "slot-7", onPress: releasing }, onOpen: open },
      { key: "b", kind: "branch", name: "Branch fix/flaky-store", text: "fix/flaky-store", released: {}, onOpen: open },
    ],
  },
  play: async ({ canvas }) => {
    releasing.mockClear();
    await userEvent.click(canvas.getByRole("button", { name: "Release slot-4" }));
    await expect(releasing).toHaveBeenCalledTimes(1);
    await expect(canvas.getByRole("img", { name: "Given back" })).toBeInTheDocument();
    await expect(canvas.getByText("Another Session holds it.")).toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: "Release fix/flaky-store" })).not.toBeInTheDocument();
  },
};

/** Nothing attached: no section is drawn, only the small picture, and no sentence. */
export const Blank: Story = {
  args: { entries: [] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("listitem")).toBeNull();
    await expect(canvas.queryByRole("heading")).toBeNull();
    await expect(canvas.getByRole("img", { name: "Nothing attached" })).toBeInTheDocument();
    await expect(canvas.getByRole("region", { name: "Attachments" }).textContent).toBe("");
    // An empty ledger draws its picture alone, with no bar of five disabled icons.
    await expect(canvas.queryByRole("group", { name: "Narrow the ledger" })).toBeNull();
    await expect(canvas.queryByRole("button")).toBeNull();
  },
};

const BAR_ENTRIES: LedgerEntry[] = [
  { key: "pr", kind: "pull_request", name: "Pull request #1843, checks passed", text: "#1843 Pin the store clock", mark: { glyph: "passed", said: "Checks passed" }, onOpen: open },
  { key: "b3", kind: "branch", name: "Branch fix/flaky-store", text: "fix/flaky-store", slot: 3, onOpen: open },
  { key: "b5", kind: "branch", name: "Branch store/migration-lock", text: <code>store/migration-lock</code>, slot: 5, onOpen: open },
  { key: "s9", kind: "slot", name: "Worktree slot 9", text: "9", onOpen: open },
];
const headingsOf = (canvas: { queryAllByRole: (role: string) => HTMLElement[] }) => canvas.queryAllByRole("heading").map((one) => one.textContent);

/**
 * The bar across the top narrows the ledger to one kind, and Search narrows every section by the words of
 * its rows. Sections run Pull requests, Branches; what waits on the person is in the thread, not here. A kind with no rows is disabled, not gone.
 */
export const FilterBar: Story = {
  args: { entries: BAR_ENTRIES },
  play: async ({ canvas }) => {
    const bar = within(canvas.getByRole("group", { name: "Narrow the ledger" }));
    await expect(headingsOf(canvas)).toEqual(["Pull requests", "Branches"]);
    // A kind narrows to its own section, and pressing it again brings every section back.
    const pulls = bar.getByRole("button", { name: "Pull requests" });
    await userEvent.click(pulls);
    await expect(pulls).toHaveAttribute("aria-pressed", "true");
    await expect(headingsOf(canvas)).toEqual(["Pull requests"]);
    await userEvent.click(pulls);
    await expect(pulls).toHaveAttribute("aria-pressed", "false");
    await expect(headingsOf(canvas)).toEqual(["Pull requests", "Branches"]);
    // One kind at a time; Branches holds the bare slot row too.
    await userEvent.click(pulls);
    await userEvent.click(bar.getByRole("button", { name: "Branches and slots" }));
    await expect(pulls).toHaveAttribute("aria-pressed", "false");
    await expect(bar.queryByRole("button", { name: "Calls" })).toBeNull();
    await expect(headingsOf(canvas)).toEqual(["Branches"]);
    await expect(canvas.getByRole("button", { name: "Open Worktree slot 9" })).toBeInTheDocument();
    // A kind the ledger holds no row of is disabled and stays in the bar.
    await expect(bar.getByRole("button", { name: "Artifacts" })).toBeDisabled();
    // Search and the kind combine; Escape clears the words and closes the field.
    await userEvent.click(bar.getByRole("button", { name: "Search" }));
    await userEvent.keyboard("FLAKY");
    await expect(canvas.getAllByRole("listitem").map((one) => one.getAttribute("aria-label"))).toEqual(["Branch fix/flaky-store"]);
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("textbox")).toBeNull();
    await expect(canvas.getAllByRole("listitem")).toHaveLength(3);
    await userEvent.click(bar.getByRole("button", { name: "Branches and slots" }));
    // Search alone reaches every section, by the words a row's text draws too.
    await userEvent.click(bar.getByRole("button", { name: "Search" }));
    await userEvent.keyboard("migration-lock");
    await expect(canvas.getAllByRole("listitem").map((one) => one.getAttribute("aria-label"))).toEqual(["Branch store/migration-lock"]);
    await userEvent.clear(canvas.getByRole("textbox", { name: "Search the ledger" }));
    await userEvent.keyboard("pin the store");
    await expect(headingsOf(canvas)).toEqual(["Pull requests"]);
    // Nothing matching draws the ledger's picture, and the bar stays.
    await userEvent.clear(canvas.getByRole("textbox", { name: "Search the ledger" }));
    await userEvent.keyboard("zzz");
    await expect(canvas.getByRole("img", { name: "Nothing attached" })).toBeInTheDocument();
    await expect(canvas.queryByRole("heading")).toBeNull();
    await expect(bar.getByRole("button", { name: "Search" })).toHaveAttribute("aria-pressed", "true");
    await userEvent.keyboard("{Escape}");
  },
};

/** Only the sections that hold a row are drawn, and the picture is gone. */
export const Some: Story = {
  args: { entries: ENTRIES.filter((one) => one.kind === "branch" || one.kind === "studio") },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("heading", { name: "Branches" })).toBeInTheDocument();
    await expect(canvas.getByRole("heading", { name: "Studios" })).toBeInTheDocument();
    await expect(canvas.queryByRole("heading", { name: "Worktree Slot" })).toBeNull();
    await expect(canvas.queryByRole("heading", { name: "Artifacts" })).toBeNull();
    await expect(canvas.queryByRole("img", { name: "Nothing attached" })).toBeNull();
  },
};

/** No Worktree Slot section: a branch row ends in its slot's chip, and a slot no branch is on is a row under Branches, after the branches. */
export const SlotChips: Story = {
  args: {
    entries: [
      { key: "s9", kind: "slot", name: "Worktree slot 9", text: "9", onOpen: open },
      { key: "b3", kind: "branch", name: "Branch fix/flaky-store", text: "fix/flaky-store", slot: 3, onOpen: open },
      { key: "b5", kind: "branch", name: "Branch store/migration-lock", text: "store/migration-lock", slot: 5, onOpen: open },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("heading", { name: "Worktree Slot" })).toBeNull();
    const branches = canvas.getByRole("region", { name: "Branches" });
    await expect(within(branches).getByRole("listitem", { name: "Branch fix/flaky-store" }).querySelector('[aria-label="Worktree slot 3"]')).not.toBeNull();
    await expect(within(branches).getByRole("listitem", { name: "Branch store/migration-lock" }).querySelector('[aria-label="Worktree slot 5"]')).not.toBeNull();
    const rows = within(branches).getAllByRole("listitem").map((one) => one.getAttribute("aria-label"));
    await expect(rows).toEqual(["Branch fix/flaky-store", "Branch store/migration-lock", "Worktree slot 9"]);
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
    await userEvent.click(canvas.getByRole("button", { name: "Open Studio Flaky store shapes" }));
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

/** Each pull request's Checks in their hue, on the mark and on the row's leading edge, so a red one is seen down the ledger. */
export const PullRequestChecks: Story = {
  args: {
    entries: [
      { key: "p1", kind: "pull_request", name: "Pull request #2112, checks passed", text: "#2112 Wire the cockpit panel", mark: { glyph: "passed", said: "Checks passed" }, onOpen: open },
      { key: "p2", kind: "pull_request", name: "Pull request #2113, checks failed", text: "#2113 Pin the store clock", mark: { glyph: "failed", said: "Checks failed: store: 2 failed" }, onOpen: open },
      { key: "p3", kind: "pull_request", name: "Pull request #2115, checks running", text: "#2115 Address the cards", mark: { glyph: "pending", said: "Checks running" }, onOpen: open },
    ],
  },
};

const pressed = fn();
const CARDS: LedgerEntry[] = [
  {
    key: "pc",
    kind: "pull_request",
    name: "Pull request #2113, checks failed",
    text: "#2113 Pin the store clock",
    mark: { glyph: "failed", said: "Checks failed: store: 2 failed" },
    card: {
      title: "#2113 Pin the store clock",
      facts: [
        { label: "Checks", value: "Failed: store: 2 failed", tone: "failed" },
        { label: "State", value: "Open, Auto-merge on" },
        { label: "Branch", value: "fix/flaky-store", mono: true },
      ],
    },
    onOpen: open,
  },
  {
    key: "st",
    kind: "studio",
    name: "Studio Flaky store shapes",
    text: "Flaky store shapes",
    mark: { glyph: "running", said: "Running" },
    card: {
      title: "Flaky store shapes",
      facts: [
        { label: "State", value: "Running", tone: "running" },
        { label: "Nodes", value: "3 nodes", mono: true },
      ],
    },
    onOpen: pressed,
  },
];

/**
 * Hovering a row, or reaching it by Tab, shows its card beside it after the tooltip's delay: what the row is, one
 * fact a line. The row stays the one stop and the one press; the marks inside it take no stop of their own.
 */
export const PreviewCards: Story = {
  name: "Preview cards",
  args: { entries: CARDS },
  play: async ({ canvas, userEvent }) => {
    const pr = canvas.getByRole("button", { name: "Open Pull request #2113, checks failed" });
    await expect(screen.queryByText("Failed: store: 2 failed")).not.toBeInTheDocument();
    await userEvent.hover(pr);
    await waitFor(() => expect(screen.getByText("Failed: store: 2 failed")).toBeVisible());
    await expect(pr).toHaveAccessibleDescription(/Checks Failed: store: 2 failed/);
    await userEvent.unhover(pr);
    await waitFor(() => expect(screen.queryByText("Failed: store: 2 failed")).not.toBeInTheDocument());
    // By keyboard: one stop a row, so nothing inside the pull request's row took one.
    // The bar's buttons are stops before the rows; start from the last of them.
    canvas.getByRole("button", { name: "Pull requests" }).focus();
    await userEvent.tab();
    await expect(pr).toHaveFocus();
    await userEvent.tab();
    const studio = canvas.getByRole("button", { name: "Open Studio Flaky store shapes" });
    await expect(studio).toHaveFocus();
    await waitFor(() => expect(screen.getByText("3 nodes")).toBeVisible());
    // The card does not take the press.
    await userEvent.keyboard("{Enter}");
    await expect(pressed).toHaveBeenCalledTimes(1);
  },
};

const openJob = fn();

/**
 * A fact that opens what it names: the merge line's chip names the Job that holds its pull request, and the Job is a press away.
 * Every other fact stays words.
 */
export const PreviewCardOpenFact: Story = {
  name: "Preview card, a fact that opens",
  render: () => (
    <LedgerPreview
      kind="pull_request"
      keys={[]}
      card={{
        title: "Who sees your config",
        where: "#2290",
        facts: [
          { label: "Checks", value: "ci running", tone: "running" },
          { label: "Branch", value: "fix/config-readers", mono: true },
          { label: "Job", value: "Who sees your config", open: openJob },
        ],
      }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    const job = canvas.getByRole("button", { name: "Open Job Who sees your config" });
    await expect(job).toHaveTextContent("Who sees your config");
    await expect(canvas.queryByRole("button", { name: /fix\/config-readers/ })).not.toBeInTheDocument();
    await userEvent.click(job);
    await expect(openJob).toHaveBeenCalledTimes(1);
  },
};
