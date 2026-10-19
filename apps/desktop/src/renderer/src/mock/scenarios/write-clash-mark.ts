// Write clashes on the Dashboard and the Sessions page. Fleet tells two live agents in one repository
// whose changes really conflict, and one agent against a main that moved; both rows carry a mark.
// Here: two Sessions on one file, two Sessions on two, a Session against main, and two Sessions
// that edit one file with no conflict, which are not marked.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import type { WriteClash } from "@armada/protocol";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const quiet = { state: "idle" } as const;

const message = (id: string, from: "agent" | "you", text: string): SessionRow => ({ id, at: "14:00:00", kind: "message", from: { kind: from }, text });
const tool = (id: string, text: string): SessionRow => ({ id, at: "14:00:00", kind: "tool", text });

const SESSIONS: Session[] = [
  {
    id: "s41",
    title: "Migration notes",
    turn: { state: "working" },
    lastTurnAt: ago(1),
    rows: [message("a0", "you", "Rename the clock column reads to written_at."), tool("a1", "Edit crates/store/src/clock.rs"), message("a2", "agent", "Renamed the reads in clock.rs.")],
    attachments: [],
  },
  {
    id: "s42",
    title: "Release script",
    turn: quiet,
    lastTurnAt: ago(25),
    rows: [message("b0", "you", "Make the clock read tolerate a missing heartbeat."), tool("b1", "Edit crates/store/src/clock.rs"), message("b2", "agent", "clock.rs now falls back to the written_at read.")],
    attachments: [],
  },
  {
    id: "s43",
    title: "Retry tuning",
    turn: { state: "working" },
    lastTurnAt: ago(2),
    rows: [tool("c0", "Edit crates/fleet/src/retry.rs"), tool("c1", "Edit crates/fleet/src/backoff.rs"), message("c2", "agent", "Both files now cap at 30 seconds.")],
    attachments: [],
  },
  {
    id: "s44",
    title: "Lint sweep",
    turn: { state: "working" },
    lastTurnAt: ago(3),
    rows: [tool("d0", "cargo clippy --workspace"), tool("d1", "Edit crates/store/src/open.rs"), message("d2", "agent", "Eleven warnings left.")],
    attachments: [],
  },
  {
    id: "s45",
    title: "Changelog tidy",
    turn: quiet,
    lastTurnAt: ago(10),
    rows: [tool("e0", "Edit docs/CHANGELOG.md"), message("e1", "agent", "Grouped the entries by release.")],
    attachments: [],
  },
  {
    id: "s46",
    title: "Release notes draft",
    turn: quiet,
    lastTurnAt: ago(15),
    rows: [tool("f0", "Edit docs/CHANGELOG.md"), message("f1", "agent", "Added the 0.9 entry at the top.")],
    attachments: [],
  },
  {
    id: "s47",
    title: "Cap the retry backoff",
    turn: { state: "working" },
    lastTurnAt: ago(4),
    rows: [tool("g0", "Edit crates/fleet/src/retry.rs"), tool("g1", "Edit crates/fleet/src/backoff.rs"), message("g2", "agent", "The backoff is capped.")],
    attachments: [],
  },
];

/** The labels are what a row calls the holder, so a tooltip names it the way the list does. */
const holder = (kind: "session", id: string, label: string) => ({ kind, id, label });
const MAIN = { kind: "main", id: "main", label: "main" } as const;

const CLASHES: readonly WriteClash[] = [
  { one: holder("session", "s41", "Migration notes"), other: holder("session", "s42", "Release script"), paths: ["crates/store/src/clock.rs"], since: ago(6) },
  { one: holder("session", "s47", "Cap the retry backoff"), other: holder("session", "s43", "Retry tuning"), paths: ["crates/fleet/src/retry.rs", "crates/fleet/src/backoff.rs"], since: ago(4) },
  { one: holder("session", "s44", "Lint sweep"), other: MAIN, paths: ["crates/store/src/open.rs"], since: ago(9) },
  // A second clash for Migration notes: its row keeps one mark, and the card names both.
  { one: holder("session", "s41", "Migration notes"), other: holder("session", "s43", "Retry tuning"), paths: ["crates/fleet/src/retry.rs"], since: ago(3) },
];

function build(): Scenario {
  const base = holding("write-clash-mark", "Write clashes: the mark on both rows of a pair, on the Dashboard, the Sessions list and the open Session");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: {
      clashes: CLASHES,
      sessions: () => sessionsStore(SESSIONS),
    },
  };
}

export const writeClashMark: Scenario = build();
