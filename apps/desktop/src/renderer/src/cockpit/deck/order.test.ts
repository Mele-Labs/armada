// How loud a Session is: amber is owed to him, red is a Check failing now and nobody has dealt with it, and the list
// reads the loudest first inside each group.

import { Bot } from "lucide-react";
import { expect, test } from "vitest";

import { fixedOrderOf, groupOf, movedIn, orderOf, toneOf } from "./order";
import type { Phase, Ship, Subject, Tone } from "./ship";

const subject = (key: string, kind: Subject["kind"], tone: Tone, phase: Phase, more: Partial<Subject> = {}): Subject =>
  ({ key, kind, id: key, title: key, root: "/r", repo: "r", mark: { Glyph: Bot, said: "state", tone }, phase, needs: false, alarm: false, peek: [], stoppable: false, repliable: false, mergeable: false, ...more }) as unknown as Subject;

test("a Session failing is red, but old news or stuck is drawn quiet", () => {
  expect(toneOf(subject("s1", "session", "bad", "needs", { idleMinutes: 3 }))).toBe("bad");
  expect(toneOf(subject("s2", "session", "bad", "needs", { idleMinutes: 15 }))).toBe("calm");
  expect(toneOf(subject("s3", "session", "bad", "held", { idleMinutes: 1 }))).toBe("calm");
});

test("what is loud is grouped as needing him, the rest by its phase, and the list reads loudest first", () => {
  const rows = [subject("calm", "session", "calm", "working"), subject("ask", "session", "ask", "needs"), subject("done", "session", "calm", "done"), subject("bad", "session", "bad", "needs", { idleMinutes: 1 }), subject("held", "session", "calm", "held")];
  expect(groupOf(rows[1]!)).toBe("needs");
  expect(groupOf(rows[0]!)).toBe("working");
  expect(groupOf(rows[4]!)).toBe("held");
  expect(orderOf({ subjects: [...rows, subject("pr", "pr", "calm", "done")] } as unknown as Ship).map((one) => one.key)).toEqual(["bad", "ask", "calm", "held", "done"]);
});

test("a reply that only tells is calm and not under Needs you, so the list does not file it with the Sessions that ask", () => {
  const tells = subject("tells", "session", "calm", "done", { reply: true });
  expect(toneOf(tells)).toBe("calm");
  expect(groupOf(tells)).not.toBe("needs");
  expect(groupOf(subject("asks", "session", "ask", "needs"))).toBe("needs");
});

const shipOf = (...subjects: Subject[]) => ({ subjects }) as unknown as Ship;
const keysOf = (rows: readonly Subject[]) => rows.map((one) => one.key);

test("rows the owner placed keep his order whatever their standing, and the ones he has not placed follow in standing order", () => {
  const ship = shipOf(subject("calm", "session", "calm", "working"), subject("ask", "session", "ask", "needs"), subject("done", "session", "calm", "done"), subject("fresh", "session", "bad", "needs", { idleMinutes: 1 }));
  expect(keysOf(fixedOrderOf(ship, ["done", "calm", "ask"]))).toEqual(["done", "calm", "ask", "fresh"]);
  // A placed key that is no row any more is passed over, and no placement at all is the standing order.
  expect(keysOf(fixedOrderOf(ship, ["gone", "calm"]))).toEqual(["calm", "fresh", "ask", "done"]);
  expect(keysOf(fixedOrderOf(ship, []))).toEqual(keysOf(orderOf(ship)));
});

test("a row whose state changes is not moved: the order is the same before and after its call is quieted", () => {
  const placed = ["calm", "ask", "done"];
  const before = shipOf(subject("calm", "session", "calm", "working"), subject("ask", "session", "ask", "needs"), subject("done", "session", "calm", "done"));
  const after = shipOf(subject("calm", "session", "calm", "working"), subject("ask", "session", "calm", "needs"), subject("done", "session", "calm", "working"));
  // Standing order does move it (the quieted row drops out of Needs you), which is what the fixed order is for.
  expect(keysOf(orderOf(before))).not.toEqual(keysOf(orderOf(after)));
  expect(keysOf(fixedOrderOf(before, placed))).toEqual(placed);
  expect(keysOf(fixedOrderOf(after, placed))).toEqual(placed);
});

test("movedIn puts a key before the target, or after it, and leaves the rest as they were", () => {
  expect(movedIn(["a", "b", "c", "d"], "d", "b", false)).toEqual(["a", "d", "b", "c"]);
  expect(movedIn(["a", "b", "c", "d"], "a", "c", true)).toEqual(["b", "c", "a", "d"]);
  expect(movedIn(["a", "b", "c"], "c", "a", true)).toEqual(["a", "c", "b"]);
});

test("movedIn changes nothing for a key or target it does not hold, or a key put beside itself", () => {
  const order = ["a", "b", "c"];
  expect(movedIn(order, "x", "b", false)).toEqual(order);
  expect(movedIn(order, "a", "x", true)).toEqual(order);
  expect(movedIn(order, "b", "b", true)).toEqual(order);
  // The order handed in is never the one handed back.
  expect(movedIn(order, "b", "b", true)).not.toBe(order);
});
