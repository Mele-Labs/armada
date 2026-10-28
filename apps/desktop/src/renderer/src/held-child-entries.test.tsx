// A Job whose agent started another, held on its start card, lists the child in the parent's Needs you: a press opens the child, and Start
// begins it from there. While the proposer still reads the child's request no landing goes, and Fleet takes the reading's.

import { expect, test, vi } from "vitest";
import type { DeliveryChoice } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { entriesOf } from "./sessions";

const AT = "2026-10-12T09:14:02.000Z";
const goes = { onGoTo: vi.fn(), onOpenLink: vi.fn(), onOpenCheck: vi.fn(), onOpenLoadout: vi.fn() };

const job = (id: string, extra: Record<string, unknown> = {}): Session => ({ id, attachments: [], rows: [], turn: { state: "idle" }, ...extra }) as unknown as Session;

const parentOf = (child: string, source = "held_child"): Session =>
  job("parent", { waitingFor: [{ id: `start:${child}`, text: `Press Start on "Retry cap"`, since: AT, source, act: { kind: "message", target: child } }] });

function rowsOf(parent: Session, others: Session[]) {
  const spies = { open: vi.fn<(id: string) => void>(), begin: vi.fn<(id: string, delivery: DeliveryChoice | undefined) => void>(), scroll: vi.fn<() => void>() };
  const rows = entriesOf(parent, goes, vi.fn(), vi.fn(), [parent, ...others], spies.open, vi.fn(), undefined, spies.scroll, { slots: [] }, undefined, undefined, undefined, undefined, spies.begin);
  return { row: rows.find((one) => one.kind === "waiting")!, ...spies };
}

test("a waiting item that messages another Job opens that Job, and does not scroll to the ask", () => {
  const { row, open, scroll } = rowsOf(parentOf("child"), [job("child", { start: { asksDelivery: true } })]);
  row.onOpen();
  expect(open).toHaveBeenCalledExactlyOnceWith("child");
  expect(scroll).not.toHaveBeenCalled();
});

test("a held_child item gets a Start that begins the child with the landing its card holds", () => {
  const { row, begin } = rowsOf(parentOf("child"), [job("child", { start: { asksDelivery: true, picked: { choice: "keep_local", from: "usual" } }, work: { proposal: { workflow: "bug" } } })]);
  expect(row.start?.starting).toBe(false);
  row.start!.onStart();
  expect(begin).toHaveBeenCalledExactlyOnceWith("child", "keep_local");
});

test("Start on a child whose card picked no landing begins it with auto land", () => {
  const { row, begin } = rowsOf(parentOf("child"), [job("child", { start: { asksDelivery: true } })]);
  row.start!.onStart();
  expect(begin).toHaveBeenCalledExactlyOnceWith("child", "auto_land");
});

test("Start on a child whose request is still being read sends no landing", () => {
  const { row, begin } = rowsOf(parentOf("child"), [job("child", { start: { asksDelivery: true, picked: { choice: "keep_local", from: "usual" } }, work: { proposal: { workflow: null, reading: true } } })]);
  row.start!.onStart();
  expect(begin).toHaveBeenCalledExactlyOnceWith("child", undefined);
});

test("Start on a child whose start asks for no delivery sends no landing", () => {
  const { row, begin } = rowsOf(parentOf("child"), [job("child", { start: { asksDelivery: false } })]);
  row.start!.onStart();
  expect(begin).toHaveBeenCalledExactlyOnceWith("child", undefined);
});

test("the row is starting while the child's start is queued, and not otherwise", () => {
  expect(rowsOf(parentOf("child"), [job("child", { start: { asksDelivery: true, queued: true } })]).row.start?.starting).toBe(true);
  expect(rowsOf(parentOf("child"), [job("child", { start: { asksDelivery: true, queued: false } })]).row.start?.starting).toBe(false);
});

test("an item that is not a held child has no Start, though it opens the Job it names", () => {
  const { row, open } = rowsOf(parentOf("child", "agent"), [job("child", { start: { asksDelivery: true } })]);
  expect(row.start).toBeUndefined();
  row.onOpen();
  expect(open).toHaveBeenCalledExactlyOnceWith("child");
});

test("a held_child item whose child has begun has no Start", () => {
  const { row } = rowsOf(parentOf("child"), [job("child")]);
  expect(row.start).toBeUndefined();
});
