// The Cockpit's Bays filter with pull requests no bay holds, each tied to the work it belongs to: a
// Job that opened one (the wire names it) and another on a Job's branch, a Session holding two, a Session
// that has ended, and one nobody in Armada owns. Over `dashboard-cockpit`; the walk `baysOutsidePrs` plays it.

import type { MergeLines } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import { SESSIONS, s206DashboardCockpit } from "./dashboard-cockpit";

const FORGE = "https://git.example/armada/pull/";
const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };

/** The Job `title` names on the Board, whose id a pull request carries on the wire. */
const jobTitled = (base: Scenario, title: string) => base.state.jobs.find((one) => one.title === title)!;

const pull = (number: number, title: string, branch: string, more: Record<string, unknown> = {}) => ({ number, title, branch, url: `${FORGE}${number}`, ...more });

/** The open pull requests the dashboard holds, and five more that sit in no bay. */
function withOutside(base: Scenario, lines: MergeLines): MergeLines {
  const migrate = jobTitled(base, "Order the store migrations");
  return {
    lines: lines.lines.map((one) => ({
      ...one,
      hub: {
        ...one.hub,
        pull_requests: [
          // The pocket repository's own pull request is on its bay, not on this line.
          ...(one.hub?.pull_requests ?? []).filter((open) => open.number !== 1895),
          // The Job that opened it, working now.
          pull(1896, "Write up the migration order", "armada/83-migrate-notes", { ci: "running", job: { id: migrate.id, title: migrate.title } }),
          // On a finished Job's branch, in the merge queue.
          pull(1901, "Fold the notification routes", "armada/91-notification-routes", { ci: "passed", queue: { state: "queued", position: 3 } }),
          // A live Session holds the first by number and the second by its branch.
          pull(1897, "Remove the unused tokens", "chore/drop-unused-tokens", { ci: "passed" }),
          pull(1898, "Rename the status tokens", "chore/rename-status-tokens", { ci: "running" }),
          // A Session that has ended.
          pull(1899, "Bump the release script", "release/script-bump", { ci: "failed" }),
        ],
      },
    })),
  };
}

function build(): Scenario {
  const base = s206DashboardCockpit;
  const landed = jobTitled(base, "Fold the two notification routes into one");
  const jobs = base.state.jobs.map((one) => (one.id === landed.id ? { ...one, branch: "armada/91-notification-routes" } : one));
  const tokens: Session = {
    ...SESSIONS.find((one) => one.id === "s12")!,
    attachments: [
      { kind: "pull_request", number: 1897, title: "Remove the unused tokens", branch: "chore/drop-unused-tokens", address: `${FORGE}1897`, checks: { state: "passed" }, state: "open", auto: false },
      { kind: "branch", name: "chore/rename-status-tokens", slot: 9 },
    ],
  };
  const release: Session = {
    ...SESSIONS.find((one) => one.id === "s11")!,
    attachments: [{ kind: "pull_request", number: 1899, title: "Bump the release script", branch: "release/script-bump", address: `${FORGE}1899`, checks: { state: "failed", failing: "release_test" }, state: "open", auto: false }],
  };
  const sessions = SESSIONS.map((one) => (one.id === "s12" ? tokens : one.id === "s11" ? release : one));
  return {
    ...base,
    name: "bays-outside-prs",
    says: "The Bays filter with pull requests outside a bay, each under the Job or Session it belongs to",
    state: { ...base.state, jobs, mergeLines: withOutside(base, base.state.mergeLines!) },
    later: [],
    draft: { ...base.draft!, sessions: (control) => sessionsStore([none, none], none, [], control, [], sessions) },
  };
}

export const s236BaysOutsidePrs: Scenario = build();
