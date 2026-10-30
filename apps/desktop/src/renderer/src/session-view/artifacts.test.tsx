// Artifacts: everything a Session made or used, read from its ledger and its thread, grouped, with the Sessions it handed a plan group to.
// Browser, since the rows carry glyphs the components package draws.

import { expect, test } from "vitest";
import type { LedgerEntry } from "@armada/components";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";

import { artifactsOf, matching } from "./artifacts";

const tool = (id: string, text: string): SessionRow => ({ id, at: "13:00:00", kind: "tool", text });

const parent = (attachments: SessionAttachment[], rows: SessionRow[] = [], plan = true): Session =>
  ({
    id: "p1",
    title: "Parent",
    attachments,
    rows,
    turn: { state: "idle" },
    ...(plan ? { work: { plan: { approved: true, scope: [], groups: [{ id: "g1", agent: "rust-engineer", title: "Name the migrations", paths: [], tasks: [] }] }, files: [{ path: "crates/fleet/src/retry.rs", act: "write" }] } } : {}),
  }) as unknown as Session;

const child = { id: "c1", title: "Migrations get names", attachments: [], rows: [], turn: { state: "working" }, standing: "working" } as unknown as Session;

const ids = (groups: ReturnType<typeof artifactsOf>) => groups.map((one) => one.id);

test("pull requests, pages, files, docs, sketches, subagents and commands are each a group, and a group with nothing is not drawn", () => {
  const session = parent(
    [
      { kind: "pull_request", number: 7, title: "Retry the lease", branch: "armada/retry", address: "https://forge/7", checks: { state: "passed" }, state: "open", auto: false },
      { kind: "artifact", form: "page", id: "https://notes/x", title: "Notes" },
      { kind: "artifact", form: "file", id: "docs/x.md", title: "X" },
      { kind: "artifact", form: "doc", id: "https://docs/d", title: "Design" },
      { kind: "subagent", id: "sa-1", task: "Find callers", state: "running", agentType: "Explore" },
      { kind: "background", id: "bg-1", form: "command", task: "Serve", command: "pnpm mock", started: "2026-10-16T13:00:00Z", state: "running" },
    ],
    [{ id: "r1", at: "13:01:00", kind: "ran", command: "git status", code: 0, tail: "" }],
  );
  expect(ids(artifactsOf(session, []))).toEqual(["pull_request", "page", "file", "doc", "subagent", "command"]);
  expect(ids(artifactsOf(parent([], [], false), []))).toEqual([]);
});

test("commits are the thread's git commit lines, newest first", () => {
  const session = parent([], [tool("a", 'Bash git commit -m "First"'), tool("b", 'Bash git commit -m "Second"')], false);
  const commits = artifactsOf(session, []).find((one) => one.id === "commit")!;
  expect(commits.rows.map((one) => one.title)).toEqual(["Second", "First"]);
});

test("files the drones touched are listed beside the ones the Session named", () => {
  const files = artifactsOf(parent([]), []).find((one) => one.id === "file")!;
  expect(files.rows.map((one) => one.title)).toEqual(["crates/fleet/src/retry.rs"]);
  expect(files.rows[0]?.sub).toBe("Written");
});

test("a running subagent is listed before a finished one", () => {
  const session = parent([
    { kind: "subagent", id: "sa-1", task: "Done one", state: "done" },
    { kind: "subagent", id: "sa-2", task: "Running one", state: "running" },
  ]);
  expect(artifactsOf(session, []).find((one) => one.id === "subagent")!.rows.map((one) => one.title)).toEqual(["Running one", "Done one"]);
});

test("the Sessions it handed a plan group to are a group, each naming its group and opening that Session", () => {
  const opened: string[] = [];
  const session = parent([{ kind: "started_to", id: "c1", group: "g1", state: "spent" } as unknown as SessionAttachment]);
  const group = artifactsOf(session, [], { sessions: [session, child], open: (id) => opened.push(id) }).find((one) => one.id === "session")!;
  expect(group.rows.map((one) => [one.title, one.sub])).toEqual([["Migrations get names", "Name the migrations"]]);
  group.rows[0]!.open?.();
  expect(opened).toEqual(["c1"]);
  expect(artifactsOf(session, []).find((one) => one.id === "session")).toBeUndefined();
});

test("a Job it started outside its plan is listed under Jobs too, with no group under its title", () => {
  const session = parent([{ kind: "started_to", id: "c1", group: "g1" }, { kind: "started_to", id: "c2" }]);
  const outside = { ...child, id: "c2", title: "Seed the test database" } as unknown as Session;
  const group = artifactsOf(session, [], { sessions: [session, child, outside], open: () => undefined }).find((one) => one.id === "session")!;
  expect(group.rows.map((one) => [one.title, one.sub])).toEqual([
    ["Migrations get names", "Name the migrations"],
    ["Seed the test database", undefined],
  ]);
});

test("the Jobs it handed plan groups to are the first group, ahead of the pull requests", () => {
  const session = parent([
    { kind: "pull_request", number: 7, title: "Retry the lease", branch: "armada/retry", address: "https://forge/7", checks: { state: "passed" }, state: "open", auto: false },
    { kind: "started_to", id: "c1", group: "g1" },
  ]);
  expect(ids(artifactsOf(session, [], { sessions: [session, child], open: () => undefined }))).toEqual(["session", "pull_request", "file"]);
});

test("a sent note is listed under Notes with its text, component, screen and state, newest first", () => {
  const note = (id: string, text: string, status: "open" | "done" | "returned", change: object = {}): SessionAttachment => ({ kind: "note", id, title: text, text, tag: "section", status, sent: "2026-10-22T10:00:00Z", ...change });
  const session = parent(
    [note("n1", "The title is cut off", "open", { component: "Deck", screen: "Cockpit", picture: "/notes/n1.png" }), note("n2", "The ledger wraps", "done"), note("n3", "The path is cut", "returned")],
    [],
    false,
  );
  const notes = artifactsOf(session, []).find((one) => one.id === "note")!;
  expect(notes.label).toBe("Notes");
  expect(notes.rows.map((one) => one.title)).toEqual(["The path is cut", "The ledger wraps", "The title is cut off"]);
  const first = notes.rows.find((one) => one.title === "The title is cut off")!;
  expect(first).toMatchObject({ sub: "Deck", words: ["Cockpit"], picture: "/notes/n1.png", mark: { glyph: "pending", said: "Open", word: "Open" } });
  expect(notes.rows.find((one) => one.title === "The ledger wraps")).toMatchObject({ mark: { glyph: "done", word: "Done" } });
  expect(notes.rows.find((one) => one.title === "The path is cut")).toMatchObject({ mark: { glyph: "stopped", word: "Returned" } });
  const bare = notes.rows.find((one) => one.title === "The ledger wraps")!;
  for (const key of ["sub", "words", "picture"]) expect(bare).not.toHaveProperty(key);
  // Typing finds a note by the screen it was written on, which the row says beside its title.
  expect(matching(artifactsOf(session, []), "cockpit").flatMap((one) => one.rows.map((row) => row.title))).toEqual(["The title is cut off"]);
});

test("a row opens through the ledger entry that knows where its press goes", () => {
  let pressed = 0;
  const session = parent([{ kind: "artifact", form: "page", id: "https://notes/x", title: "Notes" }]);
  const entry = { key: "https://notes/x", kind: "artifact", artifact: "page", name: "Notes", text: "Notes", onOpen: () => (pressed += 1) } as unknown as LedgerEntry;
  artifactsOf(session, [entry]).find((one) => one.id === "page")!.rows[0]!.open?.();
  expect(pressed).toBe(1);
});

test("typing narrows the groups to the rows it matches, or to a group whose name it matches", () => {
  const session = parent(
    [
      { kind: "artifact", form: "page", id: "https://notes/lease", title: "Lease notes" },
      { kind: "artifact", form: "file", id: "docs/other.md", title: "Other" },
    ],
    [],
    false,
  );
  const all = artifactsOf(session, []);
  expect(ids(matching(all, "lease"))).toEqual(["page"]);
  expect(ids(matching(all, "files"))).toEqual(["file"]);
  expect(ids(matching(all, "  "))).toEqual(ids(all));
  expect(matching(all, "nothing like it")).toEqual([]);
});

test("issues and links are groups of their own, a link titled by its host and path and marked by who wrote it", () => {
  const session = parent(
    [
      { kind: "issue", number: "1887", title: "store_flaky fails", address: "https://forge/issues/1887", state: "closed" },
      { kind: "link", url: "https://example.com/rfc/injectable-clocks/?utm=1", by: "you" },
      { kind: "link", url: "https://example.com/docs/instant", by: "agent" },
    ],
    [],
    false,
  );
  const groups = artifactsOf(session, []);
  expect(groups.map((one) => [one.id, one.label])).toEqual([
    ["issue", "Issues"],
    ["link", "Links"],
  ]);
  expect(groups[0]!.rows.map((one) => [one.title, one.mark?.said])).toEqual([["#1887 store_flaky fails", "Closed"]]);
  // Newest first, as every group lists.
  expect(groups[1]!.rows.map((one) => [one.title, one.sub])).toEqual([
    ["example.com/docs/instant", "Agent"],
    ["example.com/rfc/injectable-clocks", "You"],
  ]);
});
