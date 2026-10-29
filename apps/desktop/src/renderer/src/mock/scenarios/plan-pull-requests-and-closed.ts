// Two Jobs. One opens pull requests: its plan's groups stand under the chip of the one each lands in, placeholders until opened, then the
// pull request, open and then merged, as time passes. The other keeps its work local: its plan draws no chip, and its Artifacts hold a
// Closed group of what was closed from Needs you, one row to reopen and one whose call ended. The walk `plan-pull-requests-and-closed` plays it.

import type { WaitingItem } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment, WorkGroup, WorkTaskState } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { planCall } from "../plan-call";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const PAIRING = "landing-pair";
const TIDY = "landing-tidy";

const FLEET_PR = { number: 3101, title: "Serve phone pairing from Fleet" };
const BRIDGE_PR = { number: 3104, title: "Pair a phone from Bridge" };

const pull = (number: number, title: string, state: "open" | "merged"): SessionAttachment => ({
  kind: "pull_request",
  number,
  title,
  branch: "fleet/phone-pairing",
  address: `https://example.com/pull/${number}`,
  checks: { state: "passed" },
  state,
  auto: false,
});

/** The pairing plan at a stage: 0 waiting for approval, 1 approved with PR 1 open, 2 both merged. */
function pairingGroups(stage: number): WorkGroup[] {
  const done = (at: number): WorkTaskState => (stage >= at ? "done" : "open");
  const first = { label: "PR 1", ...(stage >= 1 ? { pullRequest: { ...FLEET_PR, state: stage >= 2 ? ("merged" as const) : ("open" as const) } } : {}) };
  const second = { label: "PR 2", ...(stage >= 2 ? { pullRequest: { ...BRIDGE_PR, state: "merged" as const } } : {}) };
  return [
    {
      id: "p1",
      agent: "rust-engineer",
      title: "Pairing route in Fleet",
      paths: ["crates/fleet/src/pairing"],
      landsIn: first,
      tasks: [
        { id: "P1", text: "Serve a pairing code that lasts five minutes", state: done(1) },
        { id: "P2", text: "Refuse a code used twice", state: done(1) },
      ],
    },
    {
      id: "p2",
      agent: "protocol-engineer",
      title: "Pairing code on the wire",
      paths: ["packages/protocol/src"],
      landsIn: first,
      tasks: [{ id: "P3", text: "Mirror the pairing record in TypeScript", state: done(1) }],
    },
    {
      id: "p3",
      agent: "bridge-engineer",
      title: "Pair screen in Bridge",
      paths: ["apps/desktop/src/renderer/src/pairing"],
      landsIn: second,
      tasks: [
        { id: "P4", text: "Show the code and its QR", state: done(2) },
        { id: "P5", text: "Say when the phone is paired", state: done(2) },
      ],
    },
  ];
}

function pairing(stage: number): Session {
  return {
    id: PAIRING,
    title: "Pair a phone over the local network",
    turn: { state: "idle" },
    lastTurnAt: ago(2),
    started: { delivery: "open_pr" },
    rows: [
      { id: "p-1", at: "10:02:10", kind: "message", from: { kind: "you" }, text: "Let me pair the phone from Bridge over the local network." },
      { id: "p-2", at: "10:09:40", kind: "message", from: { kind: "agent" }, text: "The Fleet route and the wire record land first, then the Bridge screen once that is merged." },
      ...(stage >= 1 ? [{ id: "p-3", at: "10:41:05", kind: "message" as const, from: { kind: "agent" as const }, text: `Opened #${FLEET_PR.number}.` }] : []),
      ...(stage >= 2 ? [{ id: "p-4", at: "11:30:12", kind: "message" as const, from: { kind: "agent" as const }, text: `#${FLEET_PR.number} and #${BRIDGE_PR.number} are merged.` }] : []),
    ],
    attachments: [...(stage >= 1 ? [pull(FLEET_PR.number, FLEET_PR.title, stage >= 2 ? "merged" : "open")] : []), ...(stage >= 2 ? [pull(BRIDGE_PR.number, BRIDGE_PR.title, "merged")] : [])],
    waitingFor: stage === 0 ? [planCall(undefined, ago(3))] : [],
    work: { plan: { scope: ["crates/fleet/src/pairing", "packages/protocol/src", "apps/desktop/src/renderer/src/pairing"], approved: stage >= 1, after: "p-2", groups: pairingGroups(stage) } },
  };
}

/** Closed from the Keep local Job's Needs you: a page still up, and a question whose call has since ended. */
const PAGE: WaitingItem = { id: "walk:https://example.com/bay-list", text: "The bay list is ready to look at", since: ago(95), source: "walk", act: { kind: "walk", target: "https://example.com/bay-list" } };
const ASKED: WaitingItem = { id: "ask-sort", text: "Sort the bays by name or by last use?", since: ago(180), source: "agent", act: { kind: "answer", target: "" } };

const tidy: Session = {
  id: TIDY,
  title: "Tidy the bay list",
  turn: { state: "idle" },
  lastTurnAt: ago(5),
  started: { delivery: "keep_local" },
  rows: [
    { id: "t-1", at: "09:12:00", kind: "message", from: { kind: "you" }, text: "Sort the bay list and drop the empty rows. Keep it on this machine." },
    { id: "t-2", at: "09:20:30", kind: "message", from: { kind: "agent" }, text: "The plan is recorded. Nothing leaves this machine." },
  ],
  attachments: [],
  waitingFor: [planCall(undefined, ago(4))],
  closed: [
    { item: PAGE, at: ago(40) },
    { item: ASKED, at: ago(150), ended: true },
  ],
  work: {
    plan: {
      scope: ["apps/desktop/src/renderer/src/cockpit/deck"],
      approved: false,
      after: "t-2",
      groups: [
        { id: "t1", agent: "bridge-engineer", title: "Sort the bay list by last use", paths: ["apps/desktop/src/renderer/src/cockpit/deck"], tasks: [{ id: "T1", text: "Sort by last use", state: "open" }] },
        { id: "t2", agent: "bridge-engineer", title: "Drop the empty rows", paths: ["apps/desktop/src/renderer/src/cockpit/deck"], tasks: [{ id: "T2", text: "Leave out a bay with no slot", state: "open" }] },
      ],
    },
  },
};

function build(): Scenario {
  const base = holding("plan-pull-requests-and-closed", "A plan whose groups stand under the pull request each lands in, and a Keep local Job's Closed items from Needs you");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: {
      sessions: () => {
        const store = sessionsStore([pairing(0), tidy], [], new Map(), []);
        let stage = 0;
        return {
          ...store,
          // Time passing opens PR 1, then merges both.
          later: () => {
            if (stage >= 2) return;
            stage += 1;
            const next = pairing(stage);
            store.edit(PAIRING, (one) => ({ ...one, rows: next.rows, attachments: next.attachments, waitingFor: next.waitingFor, work: next.work, lastTurnAt: new Date().toISOString() }));
          },
          // Reopen moves an item back to Needs you.
          reopenWaiting: (id, itemId) =>
            store.edit(id, (one) => {
              const back = one.closed?.find((closed) => closed.item.id === itemId);
              if (back === undefined) return one;
              return { ...one, closed: one.closed?.filter((closed) => closed !== back), waitingFor: [...(one.waitingFor ?? []), { ...back.item, since: new Date().toISOString() }] };
            }),
          // Closing an item keeps it under Closed.
          drop: (id, itemId, reason) => {
            const item = store.get().find((one) => one.id === id)?.waitingFor?.find((one) => one.id === itemId);
            if (item !== undefined) store.edit(id, (one) => ({ ...one, closed: [...(one.closed ?? []), { item, at: new Date().toISOString() }] }));
            store.drop(id, itemId, reason);
          },
        };
      },
    },
  };
}

export const s339PlanPullRequestsAndClosed: Scenario = build();
