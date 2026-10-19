// Two Jobs whose threads Fleet is slow to read, on a repository whose first Job's card nobody has claimed. One has never been
// read, and opens on a transcript's shape and no card until its rows arrive; the other opens on a copy read before, with a mark
// at its foot until the two newer rows land. The walk `transcript-loading` plays it. Mock data only.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const COLD = "01SESSIONTRANSCRIPTCOLDAAA";
const CACHED = "01SESSIONTRANSCRIPTCACHEDA";
/** How long Fleet takes to answer each read: under a walk's patience, so the row a `later` waits on arrives. */
const READ_MS = 4_000;

const you = (id: string, at: string, text: string) => ({ kind: "message" as const, id, at, from: { kind: "you" as const }, text });
const agent = (id: string, at: string, text: string) => ({ kind: "message" as const, id, at, from: { kind: "agent" as const }, text });

const cold = [
  you("c1", "2026-10-19T09:02:00.000Z", "store_flaky fails about one run in ten on CI. Find out why and fix it."),
  agent("c2", "2026-10-19T09:06:00.000Z", "The test reads the wall clock twice and expects both reads in the same second. On a slow runner the second read lands in the next one. Store::open will take a clock, and the test will pass it a fixed one."),
  you("c3", "2026-10-19T09:08:00.000Z", "Fine. Keep the signature of Store::open as it is."),
  agent("c4", "2026-10-19T09:15:00.000Z", "Store::open keeps its signature and Store::open_with takes the clock. store_flaky passes 200 runs in a row here."),
];

const older = [
  you("r1", "2026-10-19T08:40:00.000Z", "The reconnect backoff starts from zero after every reconnect. It should keep growing until a request succeeds."),
  agent("r2", "2026-10-19T08:47:00.000Z", "The counter lives on the connection, so each new connection starts it again. Moving it to the client."),
];

const newer = [
  you("r3", "2026-10-19T09:20:00.000Z", "Add a test that fails on the old behaviour."),
  agent("r4", "2026-10-19T09:31:00.000Z", "Added reconnect_keeps_backoff. It fails on main and passes on fix/retry-backoff."),
];

function fleet(): FakeSessionsFleet {
  const records = [
    hosted(COLD, { title: "Fix the flaky store clock test", last_turn_at: "2026-10-19T09:15:00.000Z" }),
    hosted(CACHED, { title: "Fix the retry backoff reset", last_turn_at: "2026-10-19T09:31:00.000Z" }),
  ];
  const sessions = new FakeSessionsFleet(records, { [COLD]: cold, [CACHED]: [...older, ...newer] });
  sessions.holdRead(COLD, READ_MS);
  sessions.holdRead(CACHED, READ_MS, older);
  return sessions;
}

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  return {
    ...fleet().scenario(board),
    name: "transcript-loading",
    says: "Two Jobs whose threads are slow to read: one never read, one with a copy read before and two newer rows to come",
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const s304TranscriptLoading: Scenario = build();
