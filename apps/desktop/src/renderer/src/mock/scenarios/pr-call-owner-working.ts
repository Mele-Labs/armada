// A pull request whose checks failed, held by a Session that is already working on it: no call, and no
// dot on Your move. Then the Session goes idle with nothing said to it since the failure, and the call
// comes up with the Session's thread under it. Poke sends it a line, and the call leaves again. The
// walk `prCallOwnerWorking` plays it.

import type { MergeLine } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const NUMBER = 1904;
const BRANCH = "fix/pin-store-clock";
const ADDRESS = `https://git.example/armada/pull/${NUMBER}`;

/** Its checks failed six minutes ago; every row of the thread is older, so nothing has been said to it since. */
const SESSION: Session = {
  id: "s31",
  title: "Pin the Bridge clock",
  turn: { state: "idle" },
  lastTurnAt: ago(7),
  rows: [
    { id: "k-1", at: ago(50), kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests so they stop reading the wall clock." },
    { id: "k-2", at: "13:24:30", kind: "tool", text: "Edit crates/store/tests/clock.rs" },
    { id: "k-3", at: "13:31:10", kind: "tool", text: "armada check test store" },
    { id: "k-4", at: ago(32), kind: "message", from: { kind: "agent" }, text: "The store tests read a pinned clock now. Pushed to the pull request." },
    { id: "k-5", at: "13:41:40", kind: "tool", text: "gh pr checks 1904" },
    { id: "k-6", at: ago(7), kind: "message", from: { kind: "agent" }, text: "desktop_test is failing on a clock the Bridge tests also read." },
  ],
  attachments: [
    { kind: "pull_request", number: NUMBER, title: "Pin the Bridge clock", branch: BRANCH, address: ADDRESS, checks: { state: "failed", failing: "desktop_test" }, state: "open", auto: false },
  ],
};

function build(): Scenario {
  const base = holding("pr-call-owner-working", "A pull request with failed checks held by a Session: no call while it works, the call and its thread once it is idle");
  const root = repository().root;
  const line: MergeLine = {
    root,
    line: [],
    off: [],
    landed: [],
    sent_back: [],
    hub: {
      main: { state: "green", commit: "9f1c2ab", read_at: ago(3) },
      pull_requests: [{ number: NUMBER, title: "Pin the Bridge clock", branch: BRANCH, url: ADDRESS, ci: "failed", opened_at: ago(60), updated_at: ago(6) }],
    },
  };
  return {
    ...base,
    state: { ...base.state, repository: root, mergeLines: { lines: [line] } },
    // The one moment: the Session finishes its turn. Nothing is said to it, so the failure is its call again.
    later: [{}],
    draft: {
      sessions: () => {
        const store = sessionsStore([SESSION]);
        const heard = new Set<() => void>();
        let settled = false;
        let seen: readonly Session[] | undefined;
        let shown: readonly Session[] = [];
        return {
          ...store,
          // Working until the walk lets the turn end; after that the store's own state stands, which a Poke sets working again.
          get: () => {
            const now = store.get();
            if (settled) return now;
            if (now !== seen) {
              seen = now;
              shown = now.map((one) => (one.id === SESSION.id ? { ...one, turn: { state: "working" as const } } : one));
            }
            return shown;
          },
          subscribe: (change) => {
            heard.add(change);
            const off = store.subscribe(change);
            return () => (heard.delete(change), off());
          },
          later: () => {
            settled = true;
            heard.forEach((one) => one());
          },
        };
      },
    },
  };
}

export const s237PrCallOwnerWorking: Scenario = build();
