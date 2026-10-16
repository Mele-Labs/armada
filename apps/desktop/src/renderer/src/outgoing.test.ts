// What a message of the person's does in the thread between the press and Fleet's own row.

import { expect, it } from "vitest";
import type { Session, SessionRow, SessionsDraft, SentMessage } from "@armada/screens/src/draft/sessions";

import { withOutgoing } from "./outgoing";

const message = (id: string, from: "you" | "agent", text: string): SessionRow => ({ id, at: "14:00:00", kind: "message", from: { kind: from }, text });

const session = (rows: SessionRow[]): Session => ({ id: "s1", title: "A Session", turn: { state: "idle" }, rows, attachments: [] });

const sent = (text: string): SentMessage => ({ text, files: [], sketches: [], tags: [] });

/** A draft holding one Session, whose `send` is answered by hand, and which can be given Fleet's rows. */
function held(first: SessionRow[] = [message("r1", "agent", "Hello")]) {
  let now = [session(first)];
  const listeners = new Set<() => void>();
  const answers: ((went: boolean) => void)[] = [];
  const asked: SentMessage[] = [];
  const draft = {
    get: () => now,
    subscribe: (listener: () => void) => (listeners.add(listener), () => void listeners.delete(listener)),
    send: (_id: string, one: SentMessage) => {
      asked.push(one);
      return new Promise<boolean>((done) => answers.push(done));
    },
  } as unknown as SessionsDraft;
  return {
    draft,
    asked,
    answer: (went: boolean) => answers.shift()!(went),
    /** Fleet's row arrives. */
    arrives: (row: SessionRow) => {
      now = [session([...now[0]!.rows, row])];
      listeners.forEach((one) => one());
    },
  };
}

const rowsOf = (draft: SessionsDraft) => draft.get()[0]!.rows;
const settled = () => new Promise<void>((done) => setTimeout(done, 0));

it("puts the message in the thread at once, marked as going, and takes the mark off when Fleet takes it", async () => {
  const fleet = held();
  const draft = withOutgoing(fleet.draft);
  const result = draft.send("s1", sent("Tidy the cap"));
  expect(rowsOf(draft).at(-1)).toMatchObject({ kind: "message", from: { kind: "you" }, text: "Tidy the cap", delivery: "sending" });

  fleet.answer(true);
  expect(await result).toBe(true);
  const row = rowsOf(draft).at(-1)!;
  expect(row).toMatchObject({ text: "Tidy the cap" });
  expect("delivery" in row).toBe(false);
});

it("draws Fleet's row in the message's place, under its key, and not twice", async () => {
  const fleet = held();
  const draft = withOutgoing(fleet.draft);
  const result = draft.send("s1", sent("Tidy the cap"));
  const key = rowsOf(draft).at(-1)!.id;
  fleet.answer(true);
  await result;

  fleet.arrives(message("r2", "you", "Tidy the cap"));
  const mine = rowsOf(draft).filter((row) => row.kind === "message" && row.from.kind === "you");
  expect(mine).toHaveLength(1);
  expect(mine[0]).toMatchObject({ id: "r2", stableKey: key });
});

it("finds Fleet's row before the send has answered, and leaves the message as Fleet's", async () => {
  const fleet = held();
  const draft = withOutgoing(fleet.draft);
  const result = draft.send("s1", sent("Tidy the cap"));
  fleet.arrives(message("r2", "you", "Tidy the cap"));
  expect(rowsOf(draft).filter((row) => row.kind === "message" && row.from.kind === "you")).toHaveLength(1);
  fleet.answer(true);
  await result;
  expect(rowsOf(draft).filter((row) => row.kind === "message" && row.from.kind === "you")).toHaveLength(1);
});

it("keeps a message Fleet did not take, marked, and sends the same one again on Retry", async () => {
  const fleet = held();
  const draft = withOutgoing(fleet.draft);
  const result = draft.send("s1", sent("Tidy the cap"));
  const key = rowsOf(draft).at(-1)!.id;
  fleet.answer(false);
  expect(await result).toBe(false);
  expect(rowsOf(draft).at(-1)).toMatchObject({ id: key, delivery: "failed" });

  draft.retry!("s1", key);
  expect(rowsOf(draft).at(-1)).toMatchObject({ id: key, delivery: "sending" });
  expect(fleet.asked).toHaveLength(2);
  expect(fleet.asked[1]).toBe(fleet.asked[0]);
  fleet.answer(true);
  await settled();
  expect("delivery" in rowsOf(draft).at(-1)!).toBe(false);
});

it("hands back the same Sessions when nothing of its own changed", async () => {
  const fleet = held();
  const draft = withOutgoing(fleet.draft);
  const result = draft.send("s1", sent("Tidy the cap"));
  fleet.answer(true);
  await result;
  const first = draft.get();
  expect(draft.get()).toBe(first);
});
