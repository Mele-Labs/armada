// A Job whose agent started another, held on its start card, is started from the child's Started row in the parent's thread as it is from the
// parent's Needs you: one `startOfHeldJob` builds both, so the two begin the child with the same landing and read Starting at the same time.

import { describe, expect, test, vi } from "vitest";
import type { DeliveryChoice } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { entriesOf, startOfHeldJob } from "./sessions";

const AT = "2026-10-12T09:14:02.000Z";
const goes = { onGoTo: vi.fn(), onOpenLink: vi.fn(), onOpenCheck: vi.fn(), onOpenLoadout: vi.fn() };

const job = (id: string, extra: Record<string, unknown> = {}): Session => ({ id, attachments: [], rows: [], turn: { state: "idle" }, ...extra }) as unknown as Session;

const beginSpy = () => vi.fn<(id: string, delivery: DeliveryChoice | undefined) => void>();

/** What the parent's Needs you row does when its Start is pressed, for the same child. */
function pressedInNeedsYou(child: Session) {
  const begin = beginSpy();
  const parent = job("parent", { waitingFor: [{ id: `start:${child.id}`, text: `Press Start on "Retry cap"`, since: AT, source: "held_child", act: { kind: "message", target: child.id } }] });
  const rows = entriesOf(parent, goes, vi.fn(), vi.fn(), [parent, child], vi.fn(), vi.fn(), undefined, vi.fn(), { slots: [] }, undefined, undefined, undefined, undefined, begin);
  rows.find((one) => one.kind === "waiting")!.start!.onStart();
  return begin;
}

describe("a child with no Start to give", () => {
  test("no child, or one that is not held on its start card, has none", () => {
    const begin = beginSpy();
    expect(startOfHeldJob(undefined, begin)).toBeUndefined();
    expect(startOfHeldJob(job("child"), begin)).toBeUndefined();
    expect(begin).not.toHaveBeenCalled();
  });

  test("a held child has none while the host cannot begin a Job", () => {
    expect(startOfHeldJob(job("child", { start: { asksDelivery: true } }), undefined)).toBeUndefined();
  });
});

describe("a held child's Start", () => {
  test("pressing it begins that child, and building it begins nothing", () => {
    const begin = beginSpy();
    const start = startOfHeldJob(job("child", { start: { asksDelivery: true } }), begin);
    expect(begin).not.toHaveBeenCalled();
    start!.onStart();
    expect(begin).toHaveBeenCalledTimes(1);
    expect(begin.mock.calls[0]![0]).toBe("child");
  });

  // The Needs you row of the Job that started it is the other press; each case is the landing both send.
  const cases: [string, Record<string, unknown>, DeliveryChoice | undefined][] = [
    ["the landing its card picked", { start: { asksDelivery: true, picked: { choice: "keep_local", from: "usual" } }, work: { proposal: { workflow: "bug" } } }, "keep_local"],
    ["auto land when its card picked none", { start: { asksDelivery: true } }, "auto_land"],
    ["no landing while its request is still being read", { start: { asksDelivery: true, picked: { choice: "keep_local", from: "usual" } }, work: { proposal: { workflow: null, reading: true } } }, undefined],
    ["no landing when its start asks for none", { start: { asksDelivery: false } }, undefined],
  ];
  test.each(cases)("sends %s, as the Needs you row does", (_name, extra, delivery) => {
    const child = job("child", extra);
    const begin = beginSpy();
    startOfHeldJob(child, begin)!.onStart();
    expect(begin).toHaveBeenCalledExactlyOnceWith("child", delivery);
    expect(pressedInNeedsYou(child).mock.calls).toEqual(begin.mock.calls);
  });

  test("it is starting while the child's start is queued, and not otherwise", () => {
    const starting = (queued: boolean | undefined) => startOfHeldJob(job("child", { start: { asksDelivery: true, ...(queued === undefined ? {} : { queued }) } }), beginSpy())!.starting;
    expect(starting(true)).toBe(true);
    expect(starting(false)).toBe(false);
    expect(starting(undefined)).toBe(false);
  });
});
