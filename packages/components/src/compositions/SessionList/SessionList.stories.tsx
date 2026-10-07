import type { Meta, StoryObj } from "@storybook/react-vite";
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
  jobs: [
    { id: "J52", number: 52 },
    { id: "J53", number: 53 },
  ],
  lastTurn: "14:09",
};
const NOTES: SessionRowView = {
  id: "s2",
  title: "Release notes script",
  state: "waiting",
  said: "Waiting on you",
  slots: [5],
  pullRequests: [{ number: 1847, checks: "passed", said: "Checks passed" }],
  jobs: [],
  lastTurn: "13:48",
};
const LIVE: SessionRowView = { id: "s4", title: "Docs pass", state: "working", said: "Working", slots: [8], pullRequests: [], jobs: [] };

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
    await expect(canvas.getByRole("img", { name: "Matched Pull request #1843" })).toBeInTheDocument();
  },
};

/** Many pull requests wrap on the ledger's own line; the title keeps the line above it. */
export const Crowded: Story = {
  args: {
    groups: [
      {
        label: "Running",
        rows: [
          {
            ...FLAKY,
            pullRequests: [1843, 1844, 1845, 1846, 1847, 1848, 1849, 1850].map((number) => ({ number, checks: "passed" as const, said: "Checks passed" })),
          },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    const row = canvas.getByRole("listitem", { name: "Flaky store test" });
    const title = row.querySelector(".armada-session-list__title")!.getBoundingClientRect();
    const ledger = row.querySelector(".armada-session-list__chips")!.getBoundingClientRect();
    await expect(ledger.top).toBeGreaterThanOrEqual(title.bottom);
  },
};
