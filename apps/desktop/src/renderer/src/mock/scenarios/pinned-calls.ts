// A Session with a long thread and two calls pinned under it: a question and a plan to approve. Scrolling the thread up shrinks the pinned
// region to its band; a press on the band opens it over the thread. Over the `one-session-view` scenario, with one of its Sessions given
// the rows and the question. The walk `pinnedCallsScroll` plays it.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const BIG = "one-view-4";

const turns: SessionRow[] = Array.from({ length: 16 }, (_, at): SessionRow[] => [
  { id: `pc-u${at}`, at: `13:${String(10 + at).padStart(2, "0")}:00`, kind: "message", from: { kind: "you" }, text: `Step ${at + 1} of the plan: read the files the change touches and say what you find.` },
  { id: `pc-a${at}`, at: `13:${String(10 + at).padStart(2, "0")}:30`, kind: "message", from: { kind: "agent" }, text: `Read ${at + 3} files under crates/fleet. The lease is taken in one place and released in two, and the pool is read without the lock in the retry path.` },
]).flat();

const asking = (one: Session): Session => ({
  ...one,
  rows: [...turns, ...one.rows],
  asked: {
    command: "How many attempts before it gives up?",
    call: "pc-q1",
    offers: ["allow_once", "refuse"],
    questions: [
      {
        question: "How many attempts before it gives up?",
        header: "Attempts",
        multi_select: false,
        options: [
          { label: "Three", description: "Gives up after the third try" },
          { label: "Five", description: "Gives up after the fifth try" },
        ],
      },
    ],
  },
});

export const s262PinnedCalls: Scenario = {
  ...base,
  name: "pinned-calls",
  says: "A long thread with a question and a plan to approve pinned under it",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BIG, asking);
      return store;
    },
  },
};
