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
    const { harbours } = read([slot(3, { state: "session", holder: "agent (pid 1)" }, { branch: "fix/flaky" })], [s]);
    const holder = harbours[0]!.bays[0]!.holder;
    expect(holder.kind === "session" ? holder.title : undefined).toBe("Session s1");
  });

  test("a bay held by a terminal Armada does not know, or a Session not yet named, is called by its branch", () => {
    const blank = session("s4", [{ kind: "branch", name: "fix/blank", slot: 2 }], { title: undefined });
    const { harbours } = read(
      [slot(1, { state: "session", holder: "/Users/user/.local/bin/agent (pid 7123)" }, { branch: "feat/terminal" }), slot(2, { state: "session", holder: "agent (pid 2)" }, { branch: "fix/blank" })],
      [blank],
    );
    const [terminal, unnamed] = harbours[0]!.bays.map((bay) => bay.holder);
    expect(terminal).toMatchObject({ kind: "session", title: "feat/terminal", session: undefined, by: "A terminal, pid 7123" });
    expect(unnamed).toMatchObject({ kind: "session", title: "fix/blank", by: undefined });
  });

  test("a bay a hosted Session holds under its own id draws that Session's title, never the id", () => {
    const id = "5a21fab8-f110-4b34-b646-ff5d322e7e0f";
    const s = session(id, [], { title: "Debug Bridge on Merge line" });
    const { harbours } = read([slot(1, { state: "hosted_session", session_id: id, session_title: "Served title" }, { branch: "armada/session-5a21fab8" })], [s], [tile(`session:${id}`, { owner: `session:${id}`, session: true })]);
    const holder = harbours[0]!.bays[0]!.holder;
    expect(holder).toMatchObject({ kind: "session", title: "Debug Bridge on Merge line", by: undefined });
    expect(holder.kind === "session" ? holder.session?.id : undefined).toBe(id);
    expect(holder.kind === "session" ? holder.item?.key : undefined).toBe(`session:${id}`);
  });

  test("a Session Bridge has not listed is drawn by the title Fleet served, else by its branch, and never by its id", () => {
    const id = "5a21fab8-f110-4b34-b646-ff5d322e7e0f";
    const { harbours } = read([
      slot(1, { state: "hosted_session", session_id: id, session_title: "Debug Bridge on Merge line" }, { branch: "armada/session-5a21fab8" }),
      slot(2, { state: "hosted_session", session_id: id }, { branch: "armada/session-5a21fab8" }),
      slot(3, { state: "hosted_session", session_id: id }),
    ]);
    expect(harbours[0]!.bays.map((bay) => (bay.holder.kind === "session" ? bay.holder.title : undefined))).toEqual(["Debug Bridge on Merge line", "armada/session-5a21fab8", "A Session"]);
  });

  test("a bay a hosted Session's subagent holds by a pid shows the Session it started from, not the pid", () => {
    const id = "5a21fab8-f110-4b34-b646-ff5d322e7e0f";
    const s = session(id, [{ kind: "branch", name: "armada/session-5a21fab8", slot: 8 }], { title: "Debug Bridge on Merge line" });
    const { harbours } = read([slot(10, { state: "session", holder: "node (pid 2838)", session_id: id, session_title: "Debug Bridge on Merge line" }, { branch: "fleet/main-red-session-culprit" })], [s]);
    expect(harbours[0]!.bays[0]!.holder).toMatchObject({ kind: "session", title: "Debug Bridge on Merge line", by: undefined });
  });

  test("a Session a pid names that Bridge has not listed is drawn by Fleet's title, with no terminal line", () => {
    const { harbours } = read([slot(10, { state: "session", holder: "node (pid 2838)", session_id: "gone", session_title: "Debug Bridge on Merge line" }, { branch: "fleet/x" })]);
    expect(harbours[0]!.bays[0]!.holder).toMatchObject({ kind: "session", title: "Debug Bridge on Merge line", session: undefined, by: undefined });
  });

  test("a Session working on a bay's Job rides with it, and is not listed as without a bay", () => {
    const s = session("s2", [{ kind: "job", id: "j1", number: 1, title: "Job one", state: "running", branch: "feat/a" }]);
    const { harbours, waiting } = read([slot(1, { state: "job", job_id: "j1" }, { branch: "feat/a" })], [s], [tile("j1"), tile("session:s2", { owner: "session:s2", session: true })]);
    expect(harbours[0]!.bays[0]!.riders.map((one) => one.session.id)).toEqual(["s2"]);
    expect(waiting).toEqual([]);
  });

  test("queued Jobs wait for a bay, the longest waiting first; other live work and work that is over do not", () => {
    const job = (key: string, at: string) => tile(key, { hue: "queued", at, job: { id: key } as unknown as JobSummary });
    const { waiting } = read([slot(1, { state: "free" })], [], [job("late", "2026-10-10T10:05:00Z"), job("early", "2026-10-10T10:00:00Z"), job("over", "2026-10-10T09:00:00Z"), tile("session:s3", { owner: "session:s3", hue: "queued", session: true })], [tile("over")]);
    expect(waiting.map((one) => one.key)).toEqual(["early", "late"]);
  });

  test("a closed bay with nobody in it reads closed, not free", () => {
    const { harbours } = read([slot(4, { state: "free" }, { closed: true })]);
    expect(harbours[0]!.bays[0]!.holder.kind).toBe("closed");
  });
});

describe("the trail behind a pull request", () => {
  const withRow = (row: Record<string, unknown>): MergeLines => {
    const one = lines.lines[0]!;
    return { lines: [{ ...one, hub: { pull_requests: [{ number: 1, title: "First", branch: "feat/a", url: "u/1", ci: "passed", ...row }] } }] } as unknown as MergeLines;
  };
  const trailOf = (jobs: unknown[], sessions: Session[] = [], wire: MergeLines = lines) => {
    const read = baysOf({ slots: [slot(1, { state: "free" })], jobs: jobs as JobSummary[], sessions, items: [], ended: [], views: mergeLineViews(wire, null, [], []), lines: wire, manifestOf: () => "armada", nameOf: (manifest) => manifest });
    return read.harbours[0]!.holders?.get("pull:1");
  };
  const owned = { id: "j1", title: "Job one", branch: "feat/a", created_at: "2026-10-13T08:00:00Z", started_at: "2026-10-13T08:10:00Z" };

  test("a Job's trail is the stays its row names, each with the bay and when it was given back", () => {
    const bays = [{ slot: 2, since: "2026-10-13T08:10:00Z", until: "2026-10-13T09:00:00Z" }, { slot: 5, since: "2026-10-13T09:10:00Z" }];
    expect(trailOf([{ ...owned, bays }])?.trail).toEqual({
      began: "2026-10-13T08:00:00Z",
      bays: [{ slot: 2, from: "2026-10-13T08:10:00Z", until: "2026-10-13T09:00:00Z" }, { slot: 5, from: "2026-10-13T09:10:00Z" }],
    });
  });

  test("a Job whose row names no stays was in a bay from when it started", () => {
    expect(trailOf([owned])?.trail).toEqual({ began: "2026-10-13T08:00:00Z", bays: [{ from: "2026-10-13T08:10:00Z" }] });
    expect(trailOf([{ ...owned, bays: [{ slot: 1 }, "x", null] }])?.trail?.bays).toEqual([{ from: "2026-10-13T08:10:00Z" }]);
    expect(trailOf([{ ...owned, started_at: undefined }])?.trail).toEqual({ began: "2026-10-13T08:00:00Z" });
  });

  test("a Session's trail is the one its record folded, with when this pull request was opened", () => {
    const trail = { started: "2026-10-13T07:00:00Z", bays: [{ slot: 4, from: "2026-10-13T07:05:00Z", until: "2026-10-13T07:50:00Z" }], opened: { 1: "2026-10-13T07:40:00Z", 2: "2026-10-13T07:45:00Z" } };
    const s = session("s1", [{ kind: "pull_request", number: 1, branch: "feat/a" }], { trail });
    expect(trailOf([], [s])?.trail).toEqual({ began: "2026-10-13T07:00:00Z", bays: trail.bays, opened: "2026-10-13T07:40:00Z" });
  });

  test("the forge's opened time, review, draft and conflict reach the pull request's dot", () => {
    const wire = withRow({ opened_at: "2026-10-13T07:40:00Z", review: "changes_requested", draft: true, mergeable: "conflicting" });
    const read = baysOf({ slots: [slot(1, { state: "free" })], jobs: [], sessions: [], items: [], ended: [], views: mergeLineViews(wire, null, [], []), lines: wire, manifestOf: () => "armada", nameOf: (manifest) => manifest });
    expect(read.harbours[0]!.line[0]!.card).toMatchObject({ opened: "2026-10-13T07:40:00Z", review: "changes", draft: true, conflicts: true });
  });

  test("a row that says none of it leaves the dot's card without the keys, and a forge still computing is no conflict", () => {
    const plain = baysOf({ slots: [slot(1, { state: "free" })], jobs: [], sessions: [], items: [], ended: [], views: mergeLineViews(lines, null, [], []), lines, manifestOf: () => "armada", nameOf: (manifest) => manifest });
    const card = plain.harbours[0]!.line[0]!.card;
    for (const key of ["opened", "review", "draft", "conflicts"]) expect(card).not.toHaveProperty(key);
    const wire = withRow({ mergeable: "unknown", review: "review_required" });
    const read = baysOf({ slots: [slot(1, { state: "free" })], jobs: [], sessions: [], items: [], ended: [], views: mergeLineViews(wire, null, [], []), lines: wire, manifestOf: () => "armada", nameOf: (manifest) => manifest });
    expect(read.harbours[0]!.line[0]!.card).toMatchObject({ review: "required" });
    expect(read.harbours[0]!.line[0]!.card).not.toHaveProperty("conflicts");
    const none = withRow({ review: "none" });
    expect(baysOf({ slots: [slot(1, { state: "free" })], jobs: [], sessions: [], items: [], ended: [], views: mergeLineViews(none, null, [], []), lines: none, manifestOf: () => "armada", nameOf: (manifest) => manifest }).harbours[0]!.line[0]!.card).not.toHaveProperty("review");
  });
});
