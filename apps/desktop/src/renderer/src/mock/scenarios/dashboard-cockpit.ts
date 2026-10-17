// Command Central as a cockpit: four Sessions under way and nothing asking, then two calls that
// arrive one at a time, a failed Check and a Session waiting on a command, each as a walk's `later`
// step lets time pass.

import { repository } from "@armada/screens/src/fixtures/repository";
import { mergeLines } from "@armada/screens/src/fixtures/merge-line";
import type { MergeLines, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { COCKPIT_SESSIONS } from "../cockpit-sessions";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A second repository, so there are two to read: a phone app that pairs with Fleet. */
const POCKET = "pocket";

const SESSIONS = COCKPIT_SESSIONS;
const quiet = { state: "idle" } as const;

/** Sessions as the walk's other scenarios hold them, with `s9` turning to wait on a command at the second moment. */
function asking(store: SessionsStore): SessionsStore {
  const listeners = new Set<() => void>();
  let moment = 0;
  let version = 0;
  let cached: { base: readonly Session[]; version: number; out: readonly Session[] } | undefined;
  const get = () => {
    const base = store.get();
    if (cached !== undefined && cached.base === base && cached.version === version) return cached.out;
    const out = version === 0 ? base : base.map((one) => (one.id === "s9" ? { ...one, turn: quiet, lastTurnAt: ago(0), asked: { command: "git push --force-with-lease origin fix/flaky-store" } } : one));
    cached = { base, version, out };
    return out;
  };
  return {
    ...store,
    get,
    subscribe: (onChange) => {
      listeners.add(onChange);
      const off = store.subscribe(onChange);
      return () => (listeners.delete(onChange), off());
    },
    later() {
      moment += 1;
      if (moment !== 2) return;
      version += 1;
      listeners.forEach((one) => one());
    },
  };
}

const FORGE = "https://git.example/armada/pull/";

/** The worktree pool of each repository: who holds each bay, on which branch, and since when. */
function pool(): WorktreesHeld {
  const bay = (manifest: string, n: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({
    manifest_id: manifest,
    slot: n,
    path: `/Users/user/${manifest === OWNER ? "armada" : manifest}/.armada/slots/slot-${n}`,
    base: "main",
    warm: true,
    behind: 0,
    held,
    ...rest,
  });
  return {
    slots: [
      // Hosted Sessions: this one is mid-turn, which Fleet will not release the bay of.
      bay(OWNER, 3, { state: "hosted_session", session_id: "s9", session_title: "Flaky store test" }, { branch: "fix/flaky-store", since: ago(6), behind: 2 }),
      bay(OWNER, 8, { state: "free" }, { behind: 3 }),
      bay(OWNER, 9, { state: "free" }, { behind: 5 }),
      bay(OWNER, 10, { state: "free" }, { warm: false, behind: 9 }),
      bay(OWNER, 11, { state: "free" }, { closed: true, warm: false, behind: 12 }),
      bay(OWNER, 12, { state: "free" }, { closed: true, warm: false, behind: 12 }),
      bay(OWNER, 13, { state: "unmade" }, { closed: true, warm: false }),
      // Between turns, so Release frees the bay.
      bay(OWNER, 14, { state: "hosted_session", session_id: "s12", session_title: "Theme token audit" }, { branch: "fix/theme-token-audit", since: ago(14), behind: 1 }),
    ],
  };
}

/** The forge's open pull requests: two in the merge queue, three not. One of those three is running its checks, or failing them. */
function withPulls(lines: MergeLines, failing = false): MergeLines {
  const pull = (number: number, title: string, branch: string, more: Record<string, unknown> = {}) => ({ number, title, branch, url: `${FORGE}${number}`, ...more });
  return {
    lines: lines.lines.map((one) => ({
      ...one,
      hub: {
        pull_requests: [
          pull(1893, "Bump the lockfile", "chore/bump-the-lockfile", { ci: "passed" }),
          pull(1891, "Pin the store clock", "fix/pin-store-clock", { ci: "passed", queue: { state: "queued", position: 2 } }),
          pull(1894, "Fix the reconnect wait", "fleet/reconnect-wait", { ci: failing ? "failed" : "running" }),
          pull(1890, "Gate policy on every run", "fleet/gate-policy-every-run", { ci: "passed", queue: { state: "awaiting_checks", position: 1 } }),
          pull(1895, "Read the pairing code", "pocket/pairing-code", { ci: "passed" }),
        ],
      },
    })),
  };
}

function build(): Scenario {
  const pocketManifest = { ...repository().manifest!, id: POCKET, repository: POCKET, path: "/Users/user/pocket", records_root: "/Users/user/pocket/.armada" };
  const base = holding("dashboard-cockpit", "The Dashboard with Sessions at work in two repositories, and two calls that arrive one at a time", [
    repository(),
    { root: "/Users/user/pocket", records_root: "/Users/user/pocket/.armada", manifest: pocketManifest },
  ]);
  const state = { ...base.state, repository: null, mergeLines: withPulls(mergeLines()) };
  return {
    ...base,
    state,
    held: pool(),
    later: [{ mergeLines: withPulls(mergeLines(), true) }, {}],
    draft: { sessions: () => asking(sessionsStore(SESSIONS)) },
  };
}

export const s206DashboardCockpit: Scenario = build();
