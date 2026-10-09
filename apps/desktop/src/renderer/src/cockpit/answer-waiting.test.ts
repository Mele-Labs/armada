import { describe, expect, test } from "vitest";

import { answersSent, answerWaiting } from "./answer-waiting";

type Draft = NonNullable<Parameters<typeof answerWaiting>[0]>;

/** A draft holding one Session, and recording what it was answered. */
function draftOf(asked: unknown) {
  const answered: unknown[][] = [];
  const draft = { get: () => [{ id: "s1", asked, turn: { state: "idle" }, rows: [], attachments: [] }], answer: (...args: unknown[]) => void answered.push(args) } as unknown as Draft;
  return { draft, answered };
}

const QUESTION = { question: "What should happen to it?", header: "h", multi_select: false, options: [{ label: "File an issue (Recommended)", description: "d" }, { label: "Drop it", description: "" }] };

describe("answering what a Session waits on", () => {
  test("a permission's label lands as the answer the Sessions page sends", () => {
    const { draft, answered } = draftOf({ command: "git push", call: "c1", offers: ["allow_once", "refuse"] });
    answerWaiting(draft, { session_id: "s1", item_id: "perm:c1", choice: "Allow once" });
    expect(answered).toEqual([["s1", "allow_once"]]);
  });

  test("an option of the agent's question lands as the question's answer, label whole", () => {
    const { draft, answered } = draftOf({ command: "AskUserQuestion", call: "q1", questions: [QUESTION] });
    answerWaiting(draft, { session_id: "s1", item_id: "ask:q1:0", choice: "Drop it" });
    expect(answered).toEqual([["s1", undefined, [{ question: "What should happen to it?", chosen: ["Drop it"] }]]]);
  });

  test("words typed are the person's own answer, as Other is", () => {
    const { draft, answered } = draftOf({ command: "AskUserQuestion", call: "q1", questions: [QUESTION] });
    answerWaiting(draft, { session_id: "s1", item_id: "ask:q1:0", text: "File it and fix it Friday" });
    expect(answered[0]?.[2]).toEqual([{ question: "What should happen to it?", chosen: ["File it and fix it Friday"] }]);
  });

  test("handing the decision over sends the mode, and the mock answers with the first thing offered", () => {
    const { draft, answered } = draftOf({ command: "AskUserQuestion", call: "q1", questions: [QUESTION] });
    const before = answersSent().length;
    answerWaiting(draft, { session_id: "s1", item_id: "ask:q1:0", mode: "best" });
    expect(answersSent().slice(before)).toEqual([{ session_id: "s1", item_id: "ask:q1:0", mode: "best" }]);
    expect(answered[0]?.[2]).toEqual([{ question: "What should happen to it?", chosen: ["File an issue (Recommended)"] }]);
  });
});
