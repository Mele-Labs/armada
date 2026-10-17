// How loud a Session is: amber is owed to him, red is a Check failing now and nobody has dealt with it, and the list
// reads the loudest first inside each group.

import { Bot } from "lucide-react";
import { expect, test } from "vitest";

import { groupOf, orderOf, toneOf } from "./order";
import type { Phase, Ship, Subject, Tone } from "./ship";

const subject = (key: string, kind: Subject["kind"], tone: Tone, phase: Phase, more: Partial<Subject> = {}): Subject =>
  ({ key, kind, id: key, title: key, root: "/r", repo: "r", mark: { Glyph: Bot, said: "state", tone }, phase, needs: false, alarm: false, peek: [], stoppable: false, repliable: false, mergeable: false, ...more }) as unknown as Subject;

test("a Session failing is red, but old news or stuck is drawn quiet", () => {
  expect(toneOf(subject("s1", "session", "bad", "needs", { since: "3m" }))).toBe("bad");
  expect(toneOf(subject("s2", "session", "bad", "needs", { since: "15m" }))).toBe("calm");
  expect(toneOf(subject("s3", "session", "bad", "held", { since: "1m" }))).toBe("calm");
});

test("what is loud is grouped as needing him, the rest by its phase, and the list reads loudest first", () => {
  const rows = [subject("calm", "session", "calm", "working"), subject("ask", "session", "ask", "needs"), subject("done", "session", "calm", "done"), subject("bad", "session", "bad", "needs", { since: "1m" }), subject("held", "session", "calm", "held")];
  expect(groupOf(rows[1]!)).toBe("needs");
  expect(groupOf(rows[0]!)).toBe("working");
  expect(groupOf(rows[4]!)).toBe("held");
  expect(orderOf({ subjects: [...rows, subject("pr", "pr", "calm", "done")] } as unknown as Ship).map((one) => one.key)).toEqual(["bad", "ask", "calm", "held", "done"]);
});
