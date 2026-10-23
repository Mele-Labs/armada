import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import type { LogbookEntry, LogbookState } from "@armada/protocol";

import { LogbookView, type LogbookPanel, type LogbookViewProps } from "./Logbook";

/**
 * The Logbook page: every repository's entries in panels, then Habits and This Mac, under three
 * tabs that say which state the panels hold. The open entry sits on the right.
 */
const meta: Meta<typeof LogbookView> = {
  title: "Compositions/Logbook",
  component: LogbookView,
  args: { onSave: fn(), onLock: fn(), onForget: fn() },
  decorators: [
    (Story) => (
      <div style={{ height: "40rem", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof LogbookView>;

const AT = "2026-10-20T09:00:00.000Z";
const NOW = Date.parse("2026-10-22T09:00:00.000Z");

function entry(id: string, fields: Partial<LogbookEntry> & Pick<LogbookEntry, "section" | "text">): LogbookEntry {
  return {
    id,
    kind: "convention",
    evidence: [],
    confirmed_by: 2,
    first_seen: AT,
    last_confirmed: AT,
    state: "standing",
    locked: false,
    edited: false,
    history: [],
    ...fields,
  };
}

const ENTRIES: LogbookEntry[] = [
  entry("1", {
    section: "repo",
    repository: "/Users/user/widgets",
    kind: "command",
    text: "Run specs with `bin/rspec`.",
    evidence: [{ job_id: "job-1", job_title: "Fix the flaky spec", cite: "turn 14", says: "bin/rspec passed where rake failed", at: AT }],
  }),
  entry("2", { section: "repo", repository: "/Users/user/ledger", kind: "environment", text: "Migrations run on port 5433.", locked: true }),
  entry("3", { section: "habits", kind: "habit", text: "Never run rustfmt." }),
  entry("4", { section: "repo", repository: "/Users/user/widgets", kind: "gotcha", text: "The cache survives a branch switch.", confirmed_by: 1, state: "candidate" }),
  entry("5", { section: "habits", kind: "habit", text: "Ask before touching the lockfile.", state: "faded" }),
  entry("machine-1", { section: "machine", kind: "environment", text: "Chrome is not installed." }),
];

const PANELS: { key: string; title: string; hint: string; match: (one: LogbookEntry) => boolean }[] = [
  { key: "widgets", title: "widgets", hint: "/Users/user/widgets", match: (one) => one.repository === "/Users/user/widgets" },
  { key: "ledger", title: "ledger", hint: "/Users/user/ledger", match: (one) => one.repository === "/Users/user/ledger" },
  { key: "habits", title: "Habits", hint: "How you work, across every repository", match: (one) => one.section === "habits" },
  { key: "machine", title: "This Mac", hint: "What is true of this machine", match: (one) => one.section === "machine" },
];

const panelsFor = (state: LogbookState): LogbookPanel[] =>
  PANELS.map(({ match, ...panel }) => ({ ...panel, entries: ENTRIES.filter((one) => one.state === state && match(one)) })).filter((panel) => panel.entries.length > 0);

/** The host's state, driven as the screen drives it: the tab, the open entry and whether its text is open. */
function Page(props: Pick<LogbookViewProps, "shown"> & Partial<LogbookViewProps>) {
  const { shown: first, ...rest } = props;
  const [shown, setShown] = useState<LogbookState>(first);
  const [editing, setEditing] = useState(false);
  const panels = panelsFor(shown);
  const [selected, setSelected] = useState<string | undefined>(panels[0]?.entries[0]?.id);
  return (
    <LogbookView
      panels={panels}
      shown={shown}
      onShown={(next) => {
        setShown(next);
        setSelected(panelsFor(next)[0]?.entries[0]?.id);
      }}
      selected={selected}
      onSelect={setSelected}
      editing={editing}
      onEdit={setEditing}
      onSave={() => setEditing(false)}
      onLock={() => undefined}
      onForget={() => undefined}
      now={NOW}
      {...rest}
    />
  );
}

/** In use: the entries every new Job is handed. */
export const InUse: Story = {
  render: () => <Page shown="standing" />,
};

/** New: seen in one Job, handed out once a second confirms it. */
export const New: Story = {
  render: () => <Page shown="candidate" />,
};

/** Archived: kept, no longer handed out. */
export const Archived: Story = {
  render: () => <Page shown="faded" />,
};
