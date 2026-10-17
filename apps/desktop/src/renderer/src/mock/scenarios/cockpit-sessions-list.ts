// What the Sessions page held that the Cockpit's list did not: a search by title, id, pull request, branch, slot or Job, the
// Quiet, Ended and All scopes, an ended Session opened on the stage, and the list drawn fifty rows at a time. Four live Sessions, two
// quiet, a hundred and twenty ended in the last week and one ended long ago, which only a search finds. Fleet refuses a Retro, so
// the refusal has a place to be read. The walk `cockpitSessionsList` plays it.

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { asking, FakeSessionsFleet, held, hosted, pullRequest, terminal } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-07T13:48:02.000Z";
const WORK = ["Trim the retry loop", "Rename the slot pool", "Draft the release notes", "Prune the stale branches", "Why the reader drops lines", "Fix the flaky store test"];
const TRIMMED = "01ENDEDTRIMAAAAAAAAAAAAAAA";
const BACKFILL = "01ENDEDOLDAAAAAAAAAAAAAAAA";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const records = () => [
  hosted("01ASKINGAAAAAAAAAAAAAAAAAA", { title: "Prune the stale branches", last_turn_at: minutesAgo(2), hosted: { turn: { state: "idle" }, mode: "auto", running: true, asked: asking("git push --force origin prune/stale") } }),
  hosted("01WORKINGBBBBBBBBBBBBBBBBB", { title: "Draft the migration notes", last_turn_at: minutesAgo(1), hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
  hosted("01IDLECCCCCCCCCCCCCCCCCCCC", { title: "Fix the flaky store test", last_turn_at: minutesAgo(9), attachments: [held("slot", "3"), held("branch", "fix/flaky-store"), pullRequest(1843)] }),
  hosted("01BLANKDDDDDDDDDDDDDDDDDDD"),
  terminal("01QUIETEEEEEEEEEEEEEEEEEEE", { title: "Why the reader drops lines", last_turn_at: minutesAgo(3 * 60), terminal: {} }),
  terminal("01QUIETFFFFFFFFFFFFFFFFFFF", { title: "Tidy the release notes", last_turn_at: minutesAgo(26 * 60), terminal: {} }),
  hosted(TRIMMED, { title: "Trim the retry loop", state: "ended", last_seen_at: minutesAgo(40), last_turn_at: minutesAgo(40), attachments: [pullRequest(1790, { title: "Trim the retry loop", branch: "fix/retry-loop", state: "merged" })] }),
  ...Array.from({ length: 120 }, (_, n) => {
    const at = minutesAgo(90 + n * 15);
    return hosted(`01ENDED${String(n).padStart(19, "0")}`, { title: `${WORK[n % WORK.length]} ${n + 1}`, state: "ended", last_seen_at: at, last_turn_at: at });
  }),
  hosted(BACKFILL, { title: "Backfill the audit log", state: "ended", last_seen_at: minutesAgo(20 * 24 * 60), last_turn_at: minutesAgo(20 * 24 * 60), attachments: [pullRequest(1702, { title: "Backfill the audit log", branch: "chore/audit-backfill", state: "merged" })] }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [BACKFILL]: [
    { kind: "message", id: "b1", at: AT, from: { kind: "you" }, text: "Backfill the audit log from the migration table." },
    { kind: "message", id: "b2", at: AT, from: { kind: "agent" }, text: "Backfilled forty thousand rows in batches of five hundred. Merged in #1702." },
  ],
  [TRIMMED]: [
    { kind: "message", id: "t1", at: AT, from: { kind: "you" }, text: "The retry loop spins when the lease is contended. Back off between attempts." },
    { kind: "tool", id: "t2", at: AT, text: "Read crates/fleet/src/lease.rs" },
    { kind: "message", id: "t3", at: AT, from: { kind: "agent" }, text: "Added a doubling delay up to thirty seconds. Merged in #1790." },
  ],
});

const fleet = () => {
  const served = new FakeSessionsFleet(records(), threads());
  served.refusesRetro = { code: "fleet.session_retro_being_written", message: "A retro is already being written for this Session" };
  return served;
};

function build(): Scenario {
  const board = onBoard([job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = fleet().scenario(board);
  return {
    ...served,
    name: "cockpit-sessions-list",
    says: "The Cockpit's list with a search, Quiet, Ended and All, and an ended Session open on the stage",
    // A Fleet of its own for each window, so a walk told twice starts from the same Sessions.
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const cockpitSessionsList: Scenario = build();
