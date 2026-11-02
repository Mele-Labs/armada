// A Job in panes whose agent put a pull request description to him early, then worked on for hundreds of rows, so the draft's card sits behind Show earlier.
// Needs you lists the draft, and its press opens the card as a tab of its own beside Thread. Approve, Approve with fixes and Send back settle the card and drop its
// waiting item, as Fleet does; Withdraw dismisses it, which the mock's store settles `withdrawn`. The walk `draft-far-back` plays it.

import type { DraftHow, WaitingItem } from "@armada/protocol";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const JOB = "draft-far-back";
const OPEN = "f-2";
const ITEM = `draft:${OPEN}`;
const TITLE = "Description for #57";

const TEXT = [
  "## Why",
  "",
  "The settings page listed forty options in one column, and the ones people change most were at the bottom.",
  "",
  "## How",
  "",
  "- Options are grouped under Appearance, Notifications and Privacy",
  "- Each group folds, and remembers whether it was open",
  "- Search filters the options as you type",
  "",
  "## Verified",
  "",
  "1. `pnpm test settings` passes with the new grouping cases",
  "2. Every option is still reachable from search",
].join("\n");

const WORK = ["Moved the theme picker under Appearance.", "Moved the sound toggle under Notifications.", "Renamed the history option to Keep history.", "Checked the group folds with the keyboard."];

/** Far more than the thread first draws, each its own item, so the draft is behind Show earlier. */
const LATER: SessionRow[] = Array.from({ length: 200 }, (_, at): SessionRow[] => [
  { id: `f-you-${at}`, at: "10:10:00", kind: "message", from: { kind: "you" }, text: `Next, section ${at + 1}.` },
  { id: `f-agent-${at}`, at: "10:10:30", kind: "message", from: { kind: "agent" }, text: `Section ${at + 1}: ${WORK[at % WORK.length]}` },
]).flat();

const call: WaitingItem = { id: ITEM, text: TITLE, since: new Date().toISOString(), source: "draft", act: { kind: "answer", target: ITEM }, options: [{ label: "Approve" }] };

const job: Session = {
  id: JOB,
  title: "Tidy the settings page",
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } },
  rows: [
    { id: "f-1", at: "10:02:00", kind: "message", from: { kind: "you" }, text: "Group the settings page, and draft the description for #57 first." },
    { id: OPEN, at: "10:04:00", kind: "draft", title: TITLE, text: TEXT },
    ...LATER,
  ],
  attachments: [],
  waitingFor: [call],
};

export const draftFarBack: Scenario = {
  ...s200Sessions,
  name: "draft-far-back",
  says: "A Job whose drafted pull request description is hundreds of rows back in its thread: Needs you lists it, and its press opens the card as a tab on the Job",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [job]);
      return {
        ...store,
        answerWaiting: (id, itemId, given) => {
          if (itemId !== ITEM) return store.answerWaiting?.(id, itemId, given);
          const how: DraftHow = given.fixed !== undefined ? "fixed" : given.text !== undefined ? "sent_back" : "approved";
          const words = given.fixed ?? given.text;
          const at = new Date().toISOString();
          store.edit(id, (one) => ({
            ...one,
            rows: [
              ...one.rows.map((row) => (row.id === OPEN && row.kind === "draft" ? { ...row, settled: { how, ...(words === undefined ? {} : { words }), at } } : row)),
              { id: "f-done", at: "10:40:00", kind: "message", from: { kind: "agent" }, text: how === "sent_back" ? "Understood. I will redraft #57." : "Updated #57 with the description." },
            ],
            waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ITEM),
          }));
        },
      };
    },
  },
};
