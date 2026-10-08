import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent } from "storybook/test";

import { SessionList, type SessionRowView } from "./SessionList";

/**
 * Sessions, searchable by PR number, branch, Job id, slot or title, under
 * Overview's own headings. Each row says what the Session holds, and what a
 * search matched is the ringed chip.
 */
const meta: Meta<typeof SessionList> = {
  title: "Compositions/Session list",
  component: SessionList,
  args: { query: "", onQuery: fn(), onOpen: fn(), onStart: fn() },
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 10)", padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionList>;

const FLAKY: SessionRowView = {
  id: "s7",
  title: "Flaky store test",
  state: "failing",
  said: "Checks failed on #1843",
  slots: [3],
  pullRequests: [{ number: 1843, checks: "failed", said: "Checks failed: store: 2 failed" }],
  lastTurn: "14:09",
};
const NOTES: SessionRowView = {
  id: "s2",
  title: "Release notes script",
  state: "waiting",
  said: "Waiting on you",
  slots: [5],
  pullRequests: [{ number: 1847, checks: "passed", said: "Checks passed" }],
  lastTurn: "13:48",
};
const LIVE: SessionRowView = { id: "s4", title: "Docs pass", state: "working", said: "Working", slots: [8], pullRequests: [] };

export const AtRest: Story = {
  args: {
    groups: [
      { label: "Needs you", rows: [FLAKY, NOTES] },
      { label: "Running", rows: [LIVE] },
    ],
  },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: /Flaky store test/ }));
    await expect(args.onOpen).toHaveBeenCalledWith("s7");
    await expect(canvas.getByRole("img", { name: "Working" })).toBeInTheDocument();
  },
};

/** A search found it by the pull request, and that chip is the ringed one. */
export const Found: Story = {
  args: { groups: [{ label: "Needs you", rows: [{ ...FLAKY, matched: { kind: "pull_request", number: 1843 } }] }], query: "#1843" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: /^Matched Pull request #1843/ })).toBeInTheDocument();
  },
};

const MANY = [1843, 1844, 1845, 1846, 1847, 1848, 1849, 1850, 1851, 1852, 1853, 1854].map((number) => ({ number, checks: "passed" as const, said: "Checks passed" }));

/** Many open pull requests stay on one line: the newest that fit, then a `…` that holds the rest. */
export const Crowded: Story = {
  args: { groups: [{ label: "Running", rows: [{ ...FLAKY, pullRequests: MANY }] }] },
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 7)" }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvas }) => {
    const row = canvas.getByRole("listitem", { name: "Flaky store test" });
    const title = row.querySelector(".armada-session-list__title")!.getBoundingClientRect();
    const line = row.querySelector(".armada-session-list__chips")!;
    const ledger = line.getBoundingClientRect();
    await expect(ledger.top).toBeGreaterThanOrEqual(title.bottom);
    await expect(canvas.getByRole("img", { name: /^Also open:/ }).getBoundingClientRect().right).toBeLessThanOrEqual(ledger.right);
    const more = canvas.getByRole("img", { name: /^Also open:/ });
    await expect(more).toHaveTextContent("…");
    // The newest is kept and the oldest is in the tooltip.
    await expect(canvas.getByRole("img", { name: /^Pull request #1854/ })).toBeInTheDocument();
    await expect(more.getAttribute("aria-label")).toContain("#1843");
    await expect(more.getAttribute("aria-label")).not.toContain("#1854");
  },
};

/** A merged pull request is off the row, so a row of merged ones has nothing open and no second line. */
export const NothingOpen: Story = {
  args: {
    groups: [
      {
        label: "Running",
        rows: [
          { ...LIVE, title: "All landed", slots: [], pullRequests: [{ number: 1801, checks: "passed", said: "Checks passed", state: "merged" }] },
          { ...LIVE, id: "s9", title: "One open", pullRequests: [{ number: 1802, checks: "pending", said: "Checks running", state: "open" }, { number: 1803, checks: "passed", said: "Checks passed", state: "merged" }] },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("img", { name: /^Pull request #1801/ })).toBeNull();
    await expect(canvas.queryByRole("img", { name: /^Pull request #1803/ })).toBeNull();
    const open = canvas.getByRole("img", { name: /^Pull request #1802/ });
    await expect(open).toBeInTheDocument();
    // The Checks are inside the chip, past its divider.
    await expect(open.querySelector(".armada-ref-chip__checks")).not.toBeNull();
    const row = canvas.getByRole("listitem", { name: "All landed" });
    await expect(row.querySelector(".armada-session-list__chips")).toBeNull();
    await expect(canvas.queryByRole("img", { name: /^Also open:/ })).toBeNull();
  },
};

/** The time is on the title line, at its end. */
export const TimeTopRight: Story = {
  args: { now: Date.parse("2026-10-07T14:34:00Z"), groups: [{ label: "Running", rows: [{ ...FLAKY, lastTurn: "14:30", lastTurnAt: "2026-10-07T14:30:00Z" }] }] },
  play: async ({ canvas }) => {
    const row = canvas.getByRole("listitem", { name: "Flaky store test" });
    const title = row.querySelector(".armada-session-list__title")!.getBoundingClientRect();
    const time = canvas.getByLabelText("Last turn 14:30").getBoundingClientRect();
    await expect(time.top).toBeLessThan(title.bottom);
    await expect(time.left).toBeGreaterThan(title.right);
  },
};

/** The time is how long ago the last turn was; the tooltip keeps the clock time. */
export const LastTurnAgo: Story = {
  args: {
    now: Date.parse("2026-10-07T14:34:00Z"),
    groups: [
      {
        label: "Running",
        rows: [
          { ...LIVE, id: "a", title: "Four minutes", lastTurn: "14:30", lastTurnAt: "2026-10-07T14:30:00Z" },
          { ...LIVE, id: "b", title: "Two hours", lastTurn: "12:20", lastTurnAt: "2026-10-07T12:20:00Z" },
          { ...LIVE, id: "c", title: "Three days", lastTurn: "13:15", lastTurnAt: "2026-10-04T13:15:00Z" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByLabelText("Last turn 14:30")).toHaveTextContent("4m");
    await expect(canvas.getByLabelText("Last turn 12:20")).toHaveTextContent("2h");
    await expect(canvas.getByLabelText("Last turn 13:15")).toHaveTextContent("3d");
  },
};

/** A terminal Session whose mod is older than the repository's carries a mark that names the act. */
export const ModOutOfDate: Story = {
  args: { groups: [{ label: "Running", rows: [{ ...LIVE, modOutOfDate: true }, NOTES] }] },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("img", { name: "Mod out of date: run /reload-plugins" })).toHaveLength(1);
  },
};

const VIEWS = [
  { id: "active", label: "Active" },
  { id: "quiet", label: "Quiet" },
  { id: "ended", label: "Ended" },
  { id: "all", label: "All" },
];
const BY_VIEW: Record<string, readonly string[]> = { active: ["Running"], quiet: ["Quiet"], ended: ["Ended"], all: ["Running", "Quiet", "Ended"] };
const ALL = [
  { label: "Running", rows: [LIVE] },
  { label: "Quiet", rows: [{ ...LIVE, id: "s5", title: "Why the reader drops lines", state: "quiet" as const }] },
  { label: "Ended", rows: [{ ...LIVE, id: "s6", title: "Trim the retry loop", state: "ended" as const }] },
];

/** Active on opening; Ended shows the ended rows and nothing else. */
export const Views: Story = {
  args: { groups: ALL },
  render: (args) => {
    const [view, setView] = useState("active");
    return <SessionList {...args} groups={ALL.filter((group) => BY_VIEW[view]!.includes(group.label))} views={VIEWS} view={view} onView={setView} />;
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("tab", { name: "Active", selected: true })).toBeInTheDocument();
    await expect(canvas.getByText("Docs pass")).toBeInTheDocument();
    await expect(canvas.queryByText("Trim the retry loop")).toBeNull();
    await userEvent.click(canvas.getByRole("tab", { name: "Ended" }));
    await expect(canvas.getByText("Trim the retry loop")).toBeInTheDocument();
    await expect(canvas.queryByText("Docs pass")).toBeNull();
  },
};

/** Each row has the Retro press, an icon with no label. Pressing it names that row's Session; while its retro is written the mark moves and the press holds. */
export const RetroPress: Story = {
  args: {
    onRetro: fn(),
    groups: [{ label: "Running", rows: [LIVE, { ...NOTES, retroWriting: true }] }],
  },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Retro" }));
    await expect(args.onRetro).toHaveBeenCalledWith("s4");
    await expect(canvas.getByRole("button", { name: "Writing the retro" })).toBeDisabled();
  },
};
