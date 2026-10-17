// The Cockpit's Bays filter with pull requests no bay holds, each tied to the work it belongs to and
// carrying that work's whole journey: a Session working, one idle holding two, one that has ended, a Job
// waiting on him and a Job that is done, and one pull request nobody in Armada owns. Each owner's bay is
// already given back. Over `dashboard-cockpit`; the walk `baysOutsidePrs` plays it.

import type { MergeLines } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";
import { review } from "@armada/screens/src/fixtures/build/index";

import { asRow } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import { COCKPIT_SESSIONS as SESSIONS } from "../cockpit-sessions";
import { s206DashboardCockpit } from "./dashboard-cockpit";

const OWNER = repository().manifest!.id;
const FORGE = "https://git.example/armada/pull/";
const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A Job that finished its work and is waiting on him: its bay is given back and its pull request is open. */
const asking = (() => {
  const fixture = asRow(review(), 93, "pause-column", "Add the pause marker column");
  const job = { ...fixture.job, owner_manifest_id: OWNER, created_at: ago(150), started_at: ago(140), branch: "armada/93-pause-column" };
  return { ...fixture, job };
})();

const pull = (number: number, title: string, branch: string, more: Record<string, unknown> = {}) => ({ number, title, branch, url: `${FORGE}${number}`, ...more });

/** The open pull requests the dashboard holds, and six more that sit in no bay. */
function withOutside(lines: MergeLines): MergeLines {
  return {
    lines: lines.lines.map((one) => ({
      ...one,
      hub: {
        ...one.hub,
        pull_requests: [
          // The pocket repository's own pull request is on its bay, not on this line.
          ...(one.hub?.pull_requests ?? []).filter((open) => open.number !== 1895),
          // A working Session's.
          pull(1896, "Write up the migration order", "docs/migration-order", { ci: "running" }),
          // Held by number, and by branch, by one idle Session.
          pull(1897, "Remove the unused tokens", "chore/drop-unused-tokens", { ci: "passed" }),
          pull(1898, "Rename the status tokens", "chore/rename-status-tokens", { ci: "running" }),
          // An ended Session's.
          pull(1899, "Bump the release script", "release/script-bump", { ci: "failed" }),
          // A Job's, which the wire names, and another found on a done Job's branch.
          pull(1900, "Add the pause marker column", "armada/93-pause-column", { ci: "passed", job: { id: asking.job.id, title: asking.job.title } }),
          pull(1901, "Fold the notification routes", "armada/91-notification-routes", { ci: "passed", queue: { state: "queued", position: 3 } }),
        ],
      },
    })),
  };
}

function build(): Scenario {
  const base = s206DashboardCockpit;
  const landed = base.state.jobs.find((one) => one.title === "Fold the two notification routes into one")!;
  const jobs = [
    ...base.state.jobs.map((one) => (one.id === landed.id ? { ...one, branch: "armada/91-notification-routes", created_at: ago(400), started_at: ago(390) } : one)),
    asking.job,
  ];
  const pr = (number: number, title: string, branch: string, checks: "passed" | "failed", failing = ""): SessionAttachment => ({
    kind: "pull_request",
    number,
    title,
    branch,
    address: `${FORGE}${number}`,
    checks: checks === "passed" ? { state: "passed" } : { state: "failed", failing },
    state: "open",
    auto: false,
  });
  const patched: Record<string, Partial<Session>> = {
    // Working: took bay 10 and gave it back, then went on reading.
    s10: {
      attachments: [...SESSIONS.find((one) => one.id === "s10")!.attachments, pr(1896, "Write up the migration order", "docs/migration-order", "passed")],
      trail: { started: ago(75), bays: [{ slot: 10, from: ago(70), until: ago(20) }], opened: { 1896: ago(30) } },
    },
    // Idle: one bay, two branches' worth of pull requests.
    s12: {
      attachments: [pr(1897, "Remove the unused tokens", "chore/drop-unused-tokens", "passed"), { kind: "branch", name: "chore/rename-status-tokens", slot: 9 }],
      trail: { started: ago(190), bays: [{ slot: 9, from: ago(185), until: ago(40) }], opened: { 1897: ago(95) } },
    },
    // Ended.
    s11: {
      attachments: [pr(1899, "Bump the release script", "release/script-bump", "failed", "release_test")],
      trail: { started: ago(300), bays: [{ slot: 8, from: ago(290), until: ago(160) }], opened: { 1899: ago(170) } },
    },
  };
  const sessions = SESSIONS.map((one) => (patched[one.id] === undefined ? one : { ...one, ...patched[one.id] }));
  return {
    ...base,
    name: "bays-outside-prs",
    says: "The Bays filter with pull requests outside a bay, each under the Job or Session it belongs to and the stations that work went through",
    state: { ...base.state, jobs, mergeLines: withOutside(base.state.mergeLines!) },
    reads: { ...base.reads, [asking.job.id]: asking },
    later: [],
    draft: { ...base.draft!, sessions: (control) => sessionsStore([none, none], none, control, [], sessions) },
  };
}

export const s236BaysOutsidePrs: Scenario = build();
