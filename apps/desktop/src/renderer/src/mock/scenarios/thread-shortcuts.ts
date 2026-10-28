// A Job whose thread holds three drafts waiting on him and, newest, a plan waiting for approval: the four keys on a Job's thread cards
// (`⌥↵`, `⌥E`, `⌥R`, `⌥X`) have several cards to choose between. An answered draft settles where it stands and its waiting item goes.
// The walk `thread-shortcuts` plays it.

import type { DraftHow, WaitingItem } from "@armada/protocol";
import type { Session, WorkGroup } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { planCall } from "../plan-call";
import { s200Sessions } from "./sessions";

const JOB = "thread-keys";

/** The three drafts, oldest first, by their row ids. */
const THREAD_DRAFTS = [
  { id: "k-2", title: "Description for the resumable upload change", text: "Uploads over 20 MB now resume from the last confirmed part when the connection drops." },
  { id: "k-3", title: "Release note for resumable uploads", text: "Large uploads pick up where they stopped after a dropped connection." },
  { id: "k-4", title: "Reply to the reviewer on chunk size", text: "The chunk stays at 5 MB: smaller chunks doubled the requests on a slow line." },
] as const;

const GROUPS: readonly WorkGroup[] = [
  {
    id: "u1",
    agent: "general-purpose",
    title: "Keep the upload's parts on disk",
    paths: ["src/upload"],
    tasks: [
      { id: "U1", text: "Write each confirmed part's number beside the file", state: "open" },
      { id: "U2", text: "Read it back when the upload starts again", state: "open" },
    ],
  },
  {
    id: "u2",
    agent: "general-purpose",
    title: "Show where a resumed upload stands",
    paths: ["src/upload/progress"],
    tasks: [{ id: "U3", text: "Start the progress bar at the resumed part", state: "open" }],
  },
];

const since = new Date().toISOString();
const draftCall = (id: string, title: string): WaitingItem => ({ id: `draft:${id}`, text: title, since, source: "draft", act: { kind: "answer", target: `draft:${id}` }, options: [{ label: "Approve" }] });

const job: Session = {
  id: JOB,
  title: "Resume large uploads",
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: since,
  rows: [
    { id: "k-1", at: "09:40:00", kind: "message", from: { kind: "you" }, text: "Make large uploads resume after a dropped connection, and draft the words that go with it." },
    ...THREAD_DRAFTS.map((one, at) => ({ id: one.id, at: `09:4${at + 2}:00`, kind: "draft" as const, title: one.title, text: one.text })),
    { id: "k-5", at: "09:46:00", kind: "message", from: { kind: "agent" }, text: "The three drafts are above. The plan for the change is below." },
  ],
  attachments: [],
  work: { plan: { scope: ["src/upload"], approved: false, after: "k-5", groups: GROUPS } },
  waitingFor: [...THREAD_DRAFTS.map((one) => draftCall(one.id, one.title)), planCall(undefined, since)],
};

export const threadShortcuts: Scenario = {
  ...s200Sessions,
  name: "thread-shortcuts",
  says: "A Job with three drafts and a plan waiting in its thread: the target card wears a ring and draws its keys, and ⌥↵ and ⌥X ask which card first",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [job]);
      return {
        ...store,
        answerWaiting: (id, itemId, given) => {
          const draft = THREAD_DRAFTS.find((one) => `draft:${one.id}` === itemId);
          if (id !== JOB || draft === undefined) return store.answerWaiting?.(id, itemId, given);
          const how: DraftHow = given.fixed !== undefined ? "fixed" : given.text !== undefined ? "sent_back" : "approved";
          const words = given.fixed ?? given.text;
          const at = new Date().toISOString();
          store.edit(id, (one) => ({
            ...one,
            rows: one.rows.map((row) => (row.id === draft.id && row.kind === "draft" ? { ...row, settled: { how, ...(words === undefined ? {} : { words }), at } } : row)),
            waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId),
          }));
        },
      };
    },
  },
};
