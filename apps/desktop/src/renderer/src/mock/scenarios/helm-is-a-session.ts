// Helm is a hosted Session pinned to the dock, one per repository. The dock is open on armada with
// what an earlier conversation left in it. A request starts a turn: tool calls fold into groups, the
// agent takes a slot and a branch, runs a subagent, asks a question, and opens a pull request. The
// working mark has a Stop beside it. A thought for other work still gets its card and its "Sent to" row,
// and the Sessions list shows Helm's Session with Helm's mark. The same three repositories as
// `route-a-thought`. The walk `helm-is-a-session` plays it; each `later` is the turn's next step.

import type { MergeLine, SessionRecord, WorktreesHeld } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { readAfterPause } from "../thought-reading";
import { helmRecord, withHelm } from "../sessions/helm-script";
import type { HelmStore } from "../sessions/helm-script";
import { sessionsStore } from "../sessions/script";
import { s210RouteAThought } from "./route-a-thought";

const FORGE = "https://forge.invalid/pull/";
const pull = (number: number, title: string, branch: string) => ({ number, title, branch, url: `${FORGE}${number}`, ci: "passed" });

/** armada's merge line: what is open, and the pull request Helm opens when it is done. */
const LINE: MergeLine = {
  root: repository().root,
  line: [],
  off: [],
  landed: [],
  sent_back: [],
  hub: {
    pull_requests: [
      pull(2219, "Pin the Bridge store clock", "fix/store-clock"),
      pull(2216, "Fold the two notification routes into one", "notify/one-route"),
      pull(2231, "Pin the store clock in the ledger test", "helm/pin-ledger-clock"),
    ],
  },
};

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const AT = "/Users/user/armada/.armada/slots";

/** The bay Helm takes when it first writes. */
const held = (): WorktreesHeld => ({
  slots: [
    { manifest_id: "armada", slot: 3, path: `${AT}/slot-3`, base: "main", warm: true, behind: 0, since: ago(6), branch: "sessions/list-views", held: { state: "session", holder: "Session Sessions list views" } },
    { manifest_id: "armada", slot: 8, path: `${AT}/slot-8`, base: "main", warm: true, behind: 0, since: ago(1), branch: "helm/pin-ledger-clock", held: { state: "session", holder: "Session Helm · armada" } },
  ],
});

/** The Sessions beside Helm's, as the thought reading finds them: by title and by what they hold. */
const others: Session[] = [
  {
    id: "s20",
    title: "Sessions list views",
    manifestId: "armada",
    turn: { state: "working" },
    lastTurnAt: ago(2),
    rows: [{ id: "l1", at: "14:00:12", kind: "message", from: { kind: "agent" }, text: "Reading the list's headings." }],
    attachments: [
      { kind: "slot", slot: 3 },
      { kind: "branch", name: "sessions/list-views", slot: 3 },
      { kind: "pull_request", number: 2216, title: "Sessions page: Active, Quiet and Ended views, with a status for each", branch: "sessions/list-views", address: `${FORGE}2216`, checks: { state: "pending" }, state: "draft", auto: false },
    ],
  },
  { id: "s21", title: "Cart totals rounding", manifestId: "storefront", turn: { state: "idle" }, lastTurnAt: ago(40), rows: [], attachments: [{ kind: "slot", slot: 4 }, { kind: "branch", name: "fix/cart-rounding", slot: 4 }] },
  { id: "s22", title: "Invoice PDF layout", manifestId: "billing", turn: { state: "idle" }, lastTurnAt: ago(60), rows: [], attachments: [{ kind: "slot", slot: 6 }, { kind: "branch", name: "invoice/pdf-layout", slot: 6 }] },
];

/** A Session as the thought reading reads a record: its title, its repository and what it holds. */
const recordOf = (one: Session): SessionRecord =>
  ({
    id: one.id,
    harness: "a_harness",
    origin: "bridge",
    ...(one.manifestId === undefined ? {} : { manifest_id: one.manifestId }),
    cwd: "",
    ...(one.title === undefined ? {} : { title: one.title }),
    state: one.dead === undefined ? "live" : "ended",
    started_at: ago(30),
    last_seen_at: ago(1),
    usage: {},
    attachments: one.attachments.flatMap((a) =>
      a.kind === "slot"
        ? [{ kind: "slot", target: String(a.slot), state: "standing", since: ago(30), changed_at: ago(30) }]
        : a.kind === "branch"
          ? [{ kind: "branch", target: a.name, state: "standing", since: ago(30), changed_at: ago(30) }]
          : a.kind === "pull_request"
            ? [{ kind: "pr", target: String(a.number), state: "standing", detail: { title: a.title, branch: a.branch }, since: ago(30), changed_at: ago(30) }]
            : [],
    ),
  }) as SessionRecord;

function build(): Scenario {
  const board = s210RouteAThought.state;
  // The dock answers for armada; the repository's name is as the picker draws it.
  const name = (manifestId: string | undefined) => (board.holds.repositories ?? []).find((one) => one.manifest?.id === manifestId)?.manifest?.repository ?? manifestId ?? "";
  let store: HelmStore | undefined;
  return {
    ...s210RouteAThought,
    name: "helm-is-a-session",
    says: "Helm hosted as a Session: its thread drawn as a Session's, Stop beside the working mark, the slot, branch and pull request it holds, and its row in the Sessions list",
    state: { ...board, mergeLines: { lines: [LINE] } },
    held: held(),
    behaves: () => ({
      readThought: (text) =>
        readAfterPause(text, { sessions: (store?.get() ?? []).filter((one) => one.helm !== true).map(recordOf) }),
      // Words for Helm start its turn, which the walk's `later` steps carry on.
      askHelm: async (text, _context, carry) => {
        store?.ask(text, carry);
        return { ok: true };
      },
      // Fleet keeps the row for words sent on, in Helm's own thread.
      recordThoughtSent: async (_manifestId, text, target, carry) => {
        store?.sent(text, target, carry);
        return { ok: true };
      },
    }),
    draft: {
      sessions: () => {
        const helm = helmRecord("armada", name("armada"));
        store = withHelm(sessionsStore([helm, ...others]), helm.id, name);
        return store;
      },
    },
  };
}

export const s237HelmIsASession: Scenario = build();
