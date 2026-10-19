// A bug or feature Job is stopped at its start and asked how its work is delivered, unless its first message says. The Cockpit
// has one repository chosen and Sessions served. A first message naming a delivery starts the Job with no question; any other
// leaves a `delivery` item on its list, answered with a choice, and the Job carries on. Over the quick-dispatch Cockpit. The
// walk `delivery-question` plays it.

import type { WaitingItem } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import { s210QuickDispatch as base } from "./quick-dispatch";

const ITEM = "delivery";
const clock = () => new Date().toTimeString().slice(0, 8);
let rows = 0;
const tool = (text: string) => ({ id: `d-t${(rows += 1)}`, at: clock(), kind: "tool", text }) as const;

/** The phrases Fleet reads in a first message, for the one delivery they name. */
const NAMED = /\b(open a PR|PR only|keep it local|do not push|land it|auto merge|merge it)\b/i;

const QUESTION = (): WaitingItem => ({
  id: ITEM,
  text: "How should this work be delivered?",
  since: new Date().toISOString(),
  source: "delivery",
  options: [
    { label: "Auto land", description: "Open a pull request, watch ci and merge it" },
    { label: "Open PR only", description: "Stop at the open pull request" },
    { label: "Keep local", description: "Stop at the commit on the branch, nothing pushed" },
  ],
  act: { kind: "answer", target: ITEM },
});

const owners = new Map([[repository().root, repository().manifest!.id]]);

function sessions() {
  const store = sessionsStore([], [], owners);
  return {
    ...store,
    // Fleet takes the message and holds the agent at its start: the mock's scripted reply is not run.
    send(id: string, sent: Parameters<typeof store.send>[1]) {
      const named = NAMED.test(sent.text);
      store.edit(id, (one) => ({
        ...one,
        rows: [...one.rows, { id: `d-m${(rows += 1)}`, at: clock(), kind: "message", instant: new Date().toISOString(), from: { kind: "you" }, text: sent.text }, ...(named ? [tool("Read the failing test")] : [])],
        turn: named ? { state: "working" } : { state: "idle" },
        ...(named ? {} : { waitingFor: [...(one.waitingFor ?? []), QUESTION()] }),
      }));
    },
    // The choice is kept and the Job goes on with its instructions.
    answerWaiting: (id: string, itemId: string, given: { choice?: number; text?: string }) => {
      if (itemId !== ITEM) return store.answerWaiting?.(id, itemId, given);
      store.edit(id, (one) => ({
        ...one,
        turn: { state: "working" },
        rows: [...one.rows, tool("Read the failing test")],
        waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ITEM),
      }));
    },
  };
}

export const s267DeliveryQuestion: Scenario = {
  ...base,
  name: "delivery-question",
  says: "A Job whose first message does not say how to deliver is asked, and goes on when answered; one that says starts without the question",
  state: { ...base.state, repository: repository().root },
  draft: { ...base.draft, sessions },
};
