// What a Fleet notice taught to batch or to go quiet draws as, in the thread of a window whose Fleet only simulates the rule. A real
// Fleet applies the rule where it sends the notice; the mock Fleet cannot, so the thread does it for the notices after the one taught from.

import { expect, test } from "vitest";
import type { SessionThreadRow } from "@armada/components";

import { batchNotices } from "./teach";
import type { NoticeRule } from "./teach";

const fleet = (id: string, minute: number, text: string): SessionThreadRow => ({ kind: "message", id, at: `2026-10-22T09:${minute}:00.000Z`, from: "session", sender: { id: "fleet", title: "Fleet" }, text }) as SessionThreadRow;

const THREAD: SessionThreadRow[] = [
  fleet("n1", 40, "slot-14 is 25 commits behind main."),
  fleet("n2", 50, "slot-9 is 12 commits behind main."),
  fleet("n3", 52, "#2712 failed its checks."),
  fleet("n4", 55, "slot-3 is 7 commits behind main."),
];

const AFTER = "2026-10-22T09:40:00.000Z";
const rule = (does: NoticeRule["does"]): NoticeRule => ({ name: "Slot behind main", match: "\\d+ commits behind main", does, after: AFTER });
const ids = (rows: readonly SessionThreadRow[]) => rows.map((one) => one.id);

test("a quieted notice is not drawn, and the one it was taught from and every other notice stay", () => {
  expect(ids(batchNotices(THREAD, [rule("quiet")]))).toEqual(["n1", "n3"]);
});

test("a batched notice is one row holding a line for each, named by what was taught", () => {
  const drawn = batchNotices(THREAD, [rule("batch")]);
  expect(ids(drawn)).toEqual(["n1", "n2", "n3"]);
  expect(drawn[1]).toMatchObject({ batch: "Slot behind main", text: "slot-9 is 12 commits behind main.\nslot-3 is 7 commits behind main." });
});

test("a notice taught to be told, or with no rule, is drawn as it came", () => {
  expect(batchNotices(THREAD, [rule("tell")])).toEqual(THREAD);
  expect(batchNotices(THREAD, [])).toBe(THREAD);
});
