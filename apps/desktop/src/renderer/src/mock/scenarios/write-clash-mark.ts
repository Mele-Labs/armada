// Write clashes on the Dashboard and the Sessions page. Fleet tells two live agents in one repository
// whose changes really conflict, and one agent against a main that moved; both rows carry a mark.
// Here: two Sessions on one file, a Job and a Session on two, a Session against main, and two Sessions
// that edit one file with no conflict, which are not marked. The walk `write-clash-mark` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import type { NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { WriteClash } from "../../clashes";
import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;

/** A Job on the picked repository, not yet cleared, so it stays on the Dashboard. */
function kept(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, owner_manifest_id: OWNER };
  return { ...fixture, job };
}

const cap = kept(asRow(featureRunning(), 83, "dash-cap", "Cap the retry backoff"));
const pin = kept(asRow(featureRunning(), 81, "dash-pin", "Pin the store clock"));

const STEP = { id: "implement", name: "Implement" } as const;
const DRONE = { key: "d", of: "drone", name: "Implement Drone", line: "Edit crates/fleet/src/retry.rs", step: STEP, tail: ["Read crates/fleet/src/retry.rs", "Edit crates/fleet/src/retry.rs"], state: "running" } as const;
const now: Record<string, NowView> = {
  [cap.job.id]: { running: [DRONE] },
  [pin.job.id]: { running: [{ ...DRONE, key: "d2", line: "Edit crates/store/src/clock.rs" }] },
};

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const quiet = { state: "idle" } as const;

const message = (id: string, from: "agent" | "you", text: string): SessionRow => ({ id, at: "14:00:00", kind: "message", from: { kind: from }, text });
const tool = (id: string, text: string): SessionRow => ({ id, at: "14:00:00", kind: "tool", text });

const SESSIONS: Session[] = [
  {
    id: "s41",
    title: "Migration notes",
    turn: { state: "working" },
    lastTurn: "14:05",
    lastTurnAt: ago(1),
    rows: [message("a0", "you", "Rename the clock column reads to written_at."), tool("a1", "Edit crates/store/src/clock.rs"), message("a2", "agent", "Renamed the reads in clock.rs.")],
    attachments: [],
  },
  {
    id: "s42",
    title: "Release script",
    turn: quiet,
    lastTurn: "13:40",
    lastTurnAt: ago(25),
    rows: [message("b0", "you", "Make the clock read tolerate a missing heartbeat."), tool("b1", "Edit crates/store/src/clock.rs"), message("b2", "agent", "clock.rs now falls back to the written_at read.")],
    attachments: [],
  },
  {
    id: "s43",
    title: "Retry tuning",
    turn: { state: "working" },
    lastTurn: "14:06",
    lastTurnAt: ago(2),
    rows: [tool("c0", "Edit crates/fleet/src/retry.rs"), tool("c1", "Edit crates/fleet/src/backoff.rs"), message("c2", "agent", "Both files now cap at 30 seconds.")],
    attachments: [],
  },
  {
    id: "s44",
    title: "Lint sweep",
    turn: { state: "working" },
    lastTurn: "14:04",
    lastTurnAt: ago(3),
    rows: [tool("d0", "cargo clippy --workspace"), tool("d1", "Edit crates/store/src/open.rs"), message("d2", "agent", "Eleven warnings left.")],
    attachments: [],
  },
  {
    id: "s45",
    title: "Changelog tidy",
    turn: quiet,
    lastTurn: "13:55",
    lastTurnAt: ago(10),
    rows: [tool("e0", "Edit docs/CHANGELOG.md"), message("e1", "agent", "Grouped the entries by release.")],
    attachments: [],
  },
  {
    id: "s46",
    title: "Release notes draft",
    turn: quiet,
    lastTurn: "13:50",
    lastTurnAt: ago(15),
    rows: [tool("f0", "Edit docs/CHANGELOG.md"), message("f1", "agent", "Added the 0.9 entry at the top.")],
    attachments: [],
  },
];

/** The labels are what a row calls the holder, so a tooltip names it the way the list does. */
const holder = (kind: "job" | "session", id: string, label: string) => ({ kind, id, label });
const MAIN = { kind: "main", id: "main", label: "main" } as const;

const CLASHES: readonly WriteClash[] = [
  { one: holder("session", "s41", "Migration notes"), other: holder("session", "s42", "Release script"), paths: ["crates/store/src/clock.rs"], since: ago(6) },
  { one: holder("job", cap.job.id, "Cap the retry backoff"), other: holder("session", "s43", "Retry tuning"), paths: ["crates/fleet/src/retry.rs", "crates/fleet/src/backoff.rs"], since: ago(4) },
  { one: holder("session", "s44", "Lint sweep"), other: MAIN, paths: ["crates/store/src/open.rs"], since: ago(9) },
];

function build(): Scenario {
  const base = holding("write-clash-mark", "Write clashes: the mark on both rows of a pair, on the Dashboard, the Sessions list and the open Job or Session", [pin, cap]);
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: {
      now,
      clashes: CLASHES,
      sessions: (board) => sessionsStore([none, none], none, [], board, [], SESSIONS),
    },
  };
}

export const writeClashMark: Scenario = build();
