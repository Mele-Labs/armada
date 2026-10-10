import { describe, expect, test } from "vitest";

import { mergeLineViews } from "@armada/screens";
import type { JobSummary, MergeLines, WorktreeSlot } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Item } from "../Dashboard";
import { baysOf } from "./bays";

const slot = (n: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: "armada", slot: n, path: `/slots/slot-${n}`, base: "main", warm: true, behind: 0, held, ...rest });
const job = (id: string, title: string, branch: string) => ({ id, title, branch }) as unknown as JobSummary;
const tile = (key: string, more: Partial<Item> = {}) => ({ key, title: key, hue: "running", ...more }) as unknown as Item;
const session = (id: string, attachments: unknown[], more: Partial<Session> = {}) => ({ id, title: `Session ${id}`, attachments, rows: [], turn: { state: "working" }, ...more }) as unknown as Session;
const lines: MergeLines = {
  lines: [
    {
      root: "/armada",
      line: [],
      off: [],
      landed: [],
      sent_back: [],
      hub: {
        pull_requests: [
          { number: 2, title: "Second", branch: "feat/b", url: "u/2", ci: "passed", queue: { state: "queued", position: 2 } },
          { number: 1, title: "First", branch: "feat/a", url: "u/1", ci: "passed", queue: { state: "awaiting_checks", position: 1 } },
          { number: 3, title: "Open", branch: "chore/c", url: "u/3", ci: "running" },
        ],
      },
    },
  ],
} as unknown as MergeLines;

function read(slots: WorktreeSlot[], sessions: Session[] = [], items: Item[] = [], ended: Item[] = []) {
  return baysOf({
    slots,
    jobs: [job("j1", "Job one", "feat/a"), job("j2", "Job two", "feat/x")],
    sessions,
    items,
    ended,
    views: mergeLineViews(lines, null, [], []),
    lines,
    manifestOf: () => "armada",
    nameOf: (manifest) => manifest,
  });
}

describe("the bays", () => {
  test("a bay's branch finds its pull request on the line, and a bay with none has no thread", () => {
    const { harbours } = read([slot(1, { state: "job", job_id: "j1" }, { branch: "feat/a" }), slot(2, { state: "job", job_id: "j2" }, { branch: "feat/x" })]);
    expect(harbours[0]!.bays.map((bay) => bay.dot?.card.heading)).toEqual(["#1", undefined]);
  });

  test("the line stands upright: farthest from main at the top, the next to merge at the foot", () => {
    const { harbours } = read([slot(1, { state: "free" })]);
    expect(harbours[0]!.line.map((dot) => dot.card.heading)).toEqual(["#3", "#2", "#1"]);
  });

  test("a bay held by a process is matched to the Session that names its slot", () => {
    const s = session("s1", [{ kind: "branch", name: "fix/flaky", slot: 3 }]);
    const { harbours } = read([slot(3, { state: "session", holder: "claude (pid 1)" }, { branch: "fix/flaky" })], [s]);
    const holder = harbours[0]!.bays[0]!.holder;
    expect(holder.kind === "session" ? holder.title : undefined).toBe("Session s1");
  });

  test("a Session working on a bay's Job rides with it, and is not listed as without a bay", () => {
    const s = session("s2", [{ kind: "job", id: "j1", number: 1, title: "Job one", state: "running", branch: "feat/a" }]);
    const { harbours, waiting, adrift } = read([slot(1, { state: "job", job_id: "j1" }, { branch: "feat/a" })], [s], [tile("j1"), tile("session:s2", { owner: "session:s2", session: true })]);
    expect(harbours[0]!.bays[0]!.riders.map((one) => one.session.id)).toEqual(["s2"]);
    expect([...waiting, ...adrift]).toEqual([]);
  });

  test("live work no bay holds is listed once, and work that is over is not", () => {
    const { adrift } = read([slot(1, { state: "free" })], [], [tile("j9", { owner: "j9", hue: "ask" }), tile("j9"), tile("j8"), tile("line:x", { owner: "line" })], [tile("j8")]);
    expect(adrift.map((one) => one.key)).toEqual(["j9"]);
  });

  test("queued Jobs wait for a bay, the longest waiting first, and a Session without one is only adrift", () => {
    const job = (key: string, at: string) => tile(key, { hue: "queued", at, job: { id: key } as unknown as JobSummary });
    const { waiting, adrift } = read([slot(1, { state: "free" })], [], [job("late", "2026-10-10T10:05:00Z"), job("early", "2026-10-10T10:00:00Z"), tile("session:s3", { owner: "session:s3", hue: "queued", session: true })]);
    expect(waiting.map((one) => one.key)).toEqual(["early", "late"]);
    expect(adrift.map((one) => one.key)).toEqual(["session:s3"]);
  });

  test("a closed bay with nobody in it reads closed, not free", () => {
    const { harbours } = read([slot(4, { state: "free" }, { closed: true })]);
    expect(harbours[0]!.bays[0]!.holder.kind).toBe("closed");
  });
});
