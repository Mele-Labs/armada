// The Cockpit's Bays filter with pull requests no bay holds, each tied to the work it belongs to and
// carrying that work's whole journey: a Session working, one idle holding two, one that has ended, and
// pull requests nobody in Armada owns. Each owner's bay is already given back. Over `dashboard-cockpit`; the walk `baysOutsidePrs` plays it.

import type { MergeLines } from "@armada/protocol";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import { COCKPIT_SESSIONS as SESSIONS } from "../cockpit-sessions";
import { s206DashboardCockpit } from "./dashboard-cockpit";

const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const pull = (number: number, title: string, branch: string, more: Record<string, unknown> = {}) => ({ number, title, branch, url: `${FORGE}${number}`, ...more });

/** The open pull requests the dashboard holds, and five more that sit in no bay. */
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
          // One nobody in Armada owns.
          pull(1901, "Fold the notification routes", "armada/91-notification-routes", { ci: "passed", queue: { state: "queued", position: 3 } }),
        ],
      },
    })),
  };
}

function build(): Scenario {
  const base = s206DashboardCockpit;
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
    says: "The Bays filter with pull requests outside a bay, each under the Session it belongs to and the stations that work went through",
    state: { ...base.state, mergeLines: withOutside(base.state.mergeLines!) },
    later: [],
    draft: { ...base.draft!, sessions: () => sessionsStore(sessions) },
  };
}

export const s236BaysOutsidePrs: Scenario = build();
