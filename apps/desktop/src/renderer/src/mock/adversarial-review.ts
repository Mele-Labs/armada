// A Job with plan v1 waiting, and an adversarial review of it that reads for two seconds and finds ways it breaks. The three takes of where
// the findings go (`adversarial-review-on-card`, `adversarial-review-in-needs-you`, `adversarial-review-in-plan`) are this over the
// `one-session-view` scenario, with one of its Sessions given the plan. Mock only: no Fleet serves a review yet.

import type { AdversarialReviewState, Finding } from "@armada/components";
import type { Session, WorkGroup, WorkTask } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "./moment";
import { planCall, withoutPlanCall } from "./plan-call";
import { s261OneSessionView as base } from "./scenarios/one-session-view";

const QUIET = "one-view-2";

/** How long the adversary reads before it answers. */
const READS_MS = 2000;

const task = (id: string, text: string): WorkTask => ({ id, text, state: "open", seen: false });

const GROUPS: readonly WorkGroup[] = [
  {
    id: "w1",
    agent: "rust-engineer",
    title: "Warm a bay before it is leased",
    paths: ["crates/fleet/src/pool.rs"],
    tasks: [task("W1", "Find the next free bay"), task("W2", "Warm it in the background"), task("W3", "Stop warming when the pool is full")],
  },
  {
    id: "w2",
    agent: "protocol-engineer",
    title: "Warm state on the wire",
    paths: ["crates/ipc/src/bay.rs", "packages/protocol/src/bays.ts"],
    tasks: [task("W4", "Add warming to the bay reply"), task("W5", "Mirror it in the TypeScript wire")],
  },
  {
    id: "w3",
    agent: "bridge-engineer",
    title: "Show warming on the bay tile",
    paths: ["apps/desktop/src/renderer/src/cockpit/deck"],
    tasks: [task("W6", "Draw a warming bay"), task("W7", "Say how long the warm took")],
  },
];

const FINDINGS: readonly Finding[] = [
  {
    id: "adv-1",
    consequence: "A lease waits ten minutes when every slot is held",
    expected: "The warm skips a held slot and tries the next",
    found: "\"Warm it in the background\" warms the slot the pool picked, held or not, and the lease waits on that warm to end",
    cites: ["crates/fleet/src/pool.rs:212"],
  },
  {
    id: "adv-2",
    consequence: "A bay reads as warming for good after Fleet restarts",
    expected: "A warm that was running when Fleet stopped is cleared on the next start",
    found: "\"Add warming to the bay reply\" sets the flag and nothing in the plan clears it, so it outlives the warm it described",
    cites: ["crates/ipc/src/bay.rs:48", "packages/protocol/src/bays.ts:31"],
  },
  {
    id: "adv-3",
    consequence: "Two leases can warm the same bay at once",
    expected: "One warm per bay, whoever asks first",
    found: "\"Warm it in the background\" starts a warm from each lease without asking whether one is running, so the second lease starts another over the first",
    cites: ["crates/fleet/src/pool.rs:240"],
  },
];

/**
 * Take C's findings, each about a part of the plan. The plan shows the part, so a finding does not say it again: none names a task in its
 * words. One is about a task, one about a task of another group, one about a group, one about the whole plan.
 */
const ANCHORED: readonly Finding[] = [
  {
    id: "adv-1",
    about: "W2",
    consequence: "A lease waits ten minutes when every slot is held",
    expected: "The warm skips a held slot and tries the next",
    found: "It warms the slot the pool picked, held or not, and the lease waits on that warm to end",
    cites: ["crates/fleet/src/pool.rs:212"],
  },
  {
    id: "adv-2",
    about: "W4",
    consequence: "A bay reads as warming for good after Fleet restarts",
    expected: "A warm that was running when Fleet stopped is cleared on the next start",
    found: "The flag is set and nothing in the plan clears it, so it outlives the warm it described",
    cites: ["crates/ipc/src/bay.rs:48", "packages/protocol/src/bays.ts:31"],
  },
  {
    id: "adv-3",
    about: "w3",
    consequence: "A held bay shows as warming",
    expected: "Warming is drawn only on a bay no one holds",
    found: "Neither task checks who holds the bay before it draws the mark",
    cites: ["apps/desktop/src/renderer/src/cockpit/deck/bay-tile.tsx:64"],
  },
  {
    id: "adv-4",
    consequence: "A warm that never finishes goes unnoticed",
    expected: "Each group names the check that proves it",
    found: "No task adds or changes a test, so a stuck warm passes every check",
  },
];

/** What the plan says in his words, and what each group gives him: the plain view a real plan has. */
const PLAIN = {
  summary: "The next free bay is warmed while the pool waits, so a Job that asks for one finds it ready. Fleet reports the warm to Bridge, and the bay tile shows it.",
  gives: ["A bay is warm by the time a Job asks for it", "Bridge knows which bays are warming", "The bay tile shows a warming bay and how long the warm took"],
};

const planned = (place: Place) => (one: Session): Session => ({
  ...one,
  title: "Warm the next bay",
  rows: [
    { id: "p-1", at: "13:20:00", kind: "message", from: { kind: "you" }, text: "Warm the next bay before it is leased, so a lease never waits on a cold one." },
    { id: "p-2", at: "13:41:10", kind: "message", from: { kind: "agent" }, text: "Plan v1 is recorded. Approve it, or tell me what to change." },
  ],
  waitingFor: [...withoutPlanCall(one.waitingFor), planCall(1, new Date().toISOString())],
  work: {
    workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: 0 },
    plan: {
      version: 1,
      scope: ["crates/fleet/src/pool.rs", "crates/ipc/src/bay.rs", "packages/protocol/src/bays.ts", "apps/desktop/src/renderer/src/cockpit/deck"],
      approved: false,
      after: "p-2",
      ...(place === "in-plan" ? { summary: PLAIN.summary } : {}),
      groups: place === "in-plan" ? GROUPS.map((group, at) => ({ ...group, gives: PLAIN.gives[at]! })) : GROUPS,
    },
  },
});

type Place = "card" | "needs-you" | "in-plan";

/** The scenario `name`, with the findings of the review in `place`: on the plan card, each a call in the Needs you panel, or each pinned to the part of the plan it is about. */
export function adversarialReview(name: string, says: string, place: Place): Scenario {
  return {
    ...base,
    name,
    says,
    draft: {
      ...base.draft,
      sessions: (control) => {
        const store = base.draft!.sessions!(control);
        store.edit(QUIET, planned(place));
        const timers = new Set<number>();
        const set = (review: AdversarialReviewState) => store.edit(QUIET, (one) => ({ ...one, work: { ...one.work, adversarial: { review, place } } }));
        const settle = (_id: string, finding: string, settled: "sent" | "dismissed") =>
          store.edit(QUIET, (one) => {
            const was = one.work?.adversarial;
            if (was === undefined || was.review.state !== "done") return one;
            return { ...one, work: { ...one.work, adversarial: { ...was, review: { ...was.review, findings: was.review.findings.map((it) => (it.id === finding ? { ...it, settled } : it)) } } } };
          });
        return {
          ...store,
          work: {
            ...store.work!,
            // Not yet sets the plan aside, as it does in the other plan scenarios, so the answers row is the row he will have.
            setAside: () => store.edit(QUIET, (one) => (one.work?.plan === undefined ? one : { ...one, waitingFor: withoutPlanCall(one.waitingFor), work: { ...one.work, plan: { ...one.work.plan, setAside: true } } })),
            adversarial: {
              run: () => {
                set({ state: "running" });
                const handle = window.setTimeout(() => {
                  timers.delete(handle);
                  set({ state: "done", findings: place === "in-plan" ? ANCHORED : FINDINGS });
                }, READS_MS);
                timers.add(handle);
              },
              send: (id, finding) => settle(id, finding, "sent"),
              dismiss: (id, finding) => settle(id, finding, "dismissed"),
            },
          },
          dispose: () => {
            timers.forEach((one) => window.clearTimeout(one));
            store.dispose();
          },
        };
      },
    },
  };
}
