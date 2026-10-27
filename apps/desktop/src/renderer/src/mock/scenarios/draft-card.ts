// A Job whose agent drafted a pull request description and put it to him as a card in the thread, with an earlier draft he sent back and a
// release note. Approve, Approve with fixes after Edit, and Send back after Note each settle the card and drop its waiting item. Withdraw
// dismisses the item, which the mock's store settles `withdrawn` as Fleet does. The walk `draft-card` plays it.

import type { DraftHow, WaitingItem } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const JOB = "draft-uploads";
const OPEN = "d-3";
const ITEM = `draft:${OPEN}`;
const TITLE = "Description for #412";
const NOTE_ROW = "d-6";

const BEFORE = "Retries uploads.\n\nTested by hand on a slow connection.";

const TEXT = [
  "## Why",
  "",
  "A photo upload that lost its connection halfway failed for good, and the person had to pick the photo again.",
  "",
  "## How",
  "",
  "- `uploadPhoto` retries a dropped upload three times, waiting longer each time",
  "- A retry resumes from the last part the server confirmed, not from the start",
  "- The progress bar keeps its place across a retry",
  "",
  "## Verified",
  "",
  "1. `pnpm test upload` passes with the new retry cases",
  "2. A 40 MB photo finishes after the network is cut twice",
  "3. A fourth failure shows the error and keeps the photo picked",
].join("\n");

const call: WaitingItem = { id: ITEM, text: TITLE, since: new Date().toISOString(), source: "draft", act: { kind: "answer", target: ITEM }, options: [{ label: "Approve" }] };
const note: WaitingItem = { id: `draft:${NOTE_ROW}`, text: "Release note for #412", since: new Date().toISOString(), source: "draft", act: { kind: "answer", target: `draft:${NOTE_ROW}` }, options: [{ label: "Approve" }] };

const job: Session = {
  id: JOB,
  title: "Retry dropped uploads",
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  rows: [
    { id: "d-1", at: "10:02:00", kind: "message", from: { kind: "you" }, text: "Update the description of pull request #412 so it says why uploads now retry." },
    {
      id: "d-2",
      at: "10:03:10",
      kind: "draft",
      title: "Description for #412, first try",
      text: "Retries dropped uploads.",
      settled: { how: "sent_back", words: "Too short. Say what failed before, and what you ran.", at: "10:03:50" },
    },
    { id: OPEN, at: "10:05:00", kind: "draft", title: TITLE, text: TEXT, before: BEFORE },
    { id: "d-4", at: "10:05:20", kind: "message", from: { kind: "agent" }, text: "I drafted the new description for #412." },
    { id: NOTE_ROW, at: "10:05:40", kind: "draft", title: "Release note for #412", text: "Photo uploads retry when the connection drops." },
  ],
  attachments: [],
  waitingFor: [call, note],
};

export const draftCard: Scenario = {
  ...s200Sessions,
  name: "draft-card",
  says: "A Job whose agent drafted a pull request description and a release note: a card in its thread is approved, fixed and approved, sent back, or withdrawn, and settles where it stands",
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
              {
                id: "d-5",
                at: "10:06:00",
                kind: "message",
                from: { kind: "agent" },
                text: how === "sent_back" ? "Understood. I will redraft #412." : how === "fixed" ? "Updated #412 with your text." : "Updated #412 with the description as drafted.",
              },
            ],
            waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ITEM),
          }));
        },
      };
    },
  },
};
