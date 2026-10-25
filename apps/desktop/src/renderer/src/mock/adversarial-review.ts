// A Job with plan v1 waiting, and an adversarial review of it that reads for two seconds and finds ways it breaks, each finding pinned to
// the part of the plan it is about. Over the `one-session-view` scenario, with one of its Sessions given the plan. Its presses answer
// through Fleet's routes as the mock Fleet answers them (`adversarial-fleet.ts`), and the review is drawn from Fleet's record of it
// as a wired window draws it (`adversarialOf`), so the walk plays the rules a person's press meets.

import type { AdversarialFinding, AdversarialReview } from "@armada/protocol";
import type { Session, WorkGroup, WorkTask } from "@armada/screens/src/draft/sessions";
import { adversarialOf } from "@armada/screens/src/session-work";
import type { ReviewRefused } from "@armada/screens/src/session-work";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { finished, settleFinding, startReview } from "./adversarial-fleet";
import type { Answered } from "./adversarial-fleet";

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

/**
 * The findings, each about a part of the plan. The plan shows the part, so a finding does not say it again: none names a task in its
 * words. One is about a task, one about a task of another group, one about a group, one about the whole plan.
 */
export const ANCHORED: readonly AdversarialFinding[] = [
  {
    id: "adv-1",
    about: { kind: "task", id: "W2" },
    consequence: "A lease waits ten minutes when every slot is held",
    expected: "The warm skips a held slot and tries the next",
    found: "It warms the slot the pool picked, held or not, and the lease waits on that warm to end",
    impact: "A person who starts a Job waits ten minutes, and nothing on screen says why",
    cites: ["crates/fleet/src/pool.rs:212"],
  },
  {
    id: "adv-2",
    about: { kind: "task", id: "W4" },
    consequence: "A bay reads as warming for good after Fleet restarts",
    expected: "A warm that was running when Fleet stopped is cleared on the next start",
    found: "The flag is set and nothing in the plan clears it, so it outlives the warm it described",
    impact: "After a restart the tile says warming for good, so nobody picks a bay that is ready",
    cites: ["crates/ipc/src/bay.rs:48", "packages/protocol/src/bays.ts:31"],
  },
  {
    id: "adv-3",
    about: { kind: "group", id: "w3" },
    consequence: "A held bay shows as warming",
    expected: "Warming is drawn only on a bay no one holds",
    found: "Neither task checks who holds the bay before it draws the mark",
    impact: "The bay someone is working in reads as warming, as if it were free",
    cites: ["apps/desktop/src/renderer/src/cockpit/deck/bay-tile.tsx:64"],
  },
  {
    id: "adv-4",
    about: { kind: "plan" },
    consequence: "A warm that never finishes goes unnoticed",
    expected: "Each group names the check that proves it",
    found: "No task adds or changes a test, so a stuck warm passes every check",
    impact: "Leases slow down and nobody is told",
  },
];

/** What the plan says in his words, and what each group gives him: the plain view a real plan has. */
const PLAIN = {
  summary: "The next free bay is warmed while the pool waits, so a Job that asks for one finds it ready. Fleet reports the warm to Bridge, and the bay tile shows it.",
  gives: ["A bay is warm by the time a Job asks for it", "Bridge knows which bays are warming", "The bay tile shows a warming bay and how long the warm took"],
};

const planned = (one: Session): Session => ({
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
      summary: PLAIN.summary,
      groups: GROUPS.map((group, at) => ({ ...group, gives: PLAIN.gives[at]! })),
    },
  },
});

/** The scenario `name`, with the findings of the review each pinned to the part of the plan it is about. */
export function adversarialReview(name: string, says: string): Scenario {
  return {
    ...base,
    name,
    says,
    draft: {
      ...base.draft,
      sessions: (control) => {
        const store = base.draft!.sessions!(control);
        store.edit(QUIET, planned);
        const timers = new Set<number>();
        // Fleet's record of the review, and its refusal of the last press: the card draws both as a wired window does.
        let kept: AdversarialReview | undefined;
        let refused: ReviewRefused | undefined;
        const version = () => store.get().find((one) => one.id === QUIET)?.work?.plan?.version ?? 1;
        const draw = () => store.edit(QUIET, (one) => {
          const drawn = adversarialOf({ version: version(), ...(kept === undefined ? {} : { adversarial: kept }) }, refused);
          const { adversarial: _was, ...work } = one.work ?? {};
          return { ...one, work: drawn === undefined ? work : { ...work, adversarial: drawn } };
        });
        const answered = (answer: Answered) => {
          if (answer.ok) kept = answer.review;
          else refused = { version: version(), said: refusalWords(answer) };
          draw();
          return answer;
        };
        const subject = (id: string) => ({ kind: "plan" as const, session_id: id, version: version() });
        return {
          ...store,
          work: {
            ...store.work!,
            // Not yet sets the plan aside, as it does in the other plan scenarios, so the answers row is the row he will have.
            setAside: () => store.edit(QUIET, (one) => (one.work?.plan === undefined ? one : { ...one, waitingFor: withoutPlanCall(one.waitingFor), work: { ...one.work, plan: { ...one.work.plan, setAside: true } } })),
            adversarial: {
              run: (id) => {
                refused = undefined;
                const answer = answered(startReview({ version: version(), ...(kept === undefined ? {} : { adversarial: kept }) }, { subject: subject(id), ...(kept === undefined ? {} : { again: true }) }, new Date().toISOString()));
                if (!answer.ok || answer.review.state !== "running") return;
                const begun = answer.review;
                const handle = window.setTimeout(() => {
                  timers.delete(handle);
                  if (kept !== begun) return;
                  kept = finished(begun, ANCHORED, new Date().toISOString());
                  draw();
                }, READS_MS);
                timers.add(handle);
              },
              send: (id, finding) => void answered(settleFinding({ version: version(), ...(kept === undefined ? {} : { adversarial: kept }) }, { subject: subject(id), finding_id: finding }, "sent", new Date().toISOString())),
              dismiss: (id, finding) => void answered(settleFinding({ version: version(), ...(kept === undefined ? {} : { adversarial: kept }) }, { subject: subject(id), finding_id: finding }, "dismissed", new Date().toISOString())),
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
