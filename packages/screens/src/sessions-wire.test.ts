import { describe, expect, it } from "vitest";

import type { Attachment, JobSummary, SessionRecord, SessionRow } from "@armada/protocol";
import { job } from "./fixtures/build/base";
import { ENDED_LISTED_DAYS, attachmentsOfRecord, cleanTitle, jobStateOf, numberOf, rowsOfThread, sessionOfRecord, sessionsOfRecords } from "./sessions-wire";
import type { Beside } from "./sessions-wire";
import { ownerOf, sessionsMatching } from "./draft/sessions";

const AT = "2026-10-07T13:48:02Z";

const held = (kind: string, target: string, detail: Record<string, string> = {}, state: Attachment["state"] = "standing", manifest = "armada"): Attachment => ({
  kind,
  manifest_id: manifest,
  target,
  state,
  detail,
  since: AT,
  changed_at: AT,
});

const record = (id: string, change: Partial<SessionRecord> = {}): SessionRecord => ({
  id,
  harness: "a_harness",
  origin: "bridge",
  manifest_id: "armada",
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  hosted: { turn: { state: "idle" }, mode: "auto", running: false },
  ...change,
});

const beside = (jobs: JobSummary[] = []): Beside => ({ jobs, picture: () => undefined, sketches: [], pending: [] });

describe("the ledger", () => {
  it("pairs a branch with the slot the session holds in the same repository", () => {
    const attachments = attachmentsOfRecord(
      record("a", { attachments: [held("slot", "2", {}, "standing", "storefront"), held("slot", "3"), held("branch", "fix/flaky-store")] }),
      beside(),
    );
    expect(attachments).toContainEqual({ kind: "branch", name: "fix/flaky-store", slot: 3 });
    expect(attachments).toContainEqual({ kind: "slot", slot: 3 });
  });

  it("leaves out what a session let go, and keeps a merged pull request as merged", () => {
    const attachments = attachmentsOfRecord(
      record("a", {
        attachments: [
          held("slot", "3", {}, "given_back"),
          held("pr", "1847", { state: "merged", title: "Pin the clock", checks: "passed" }, "spent"),
          held("pr", "1850", { title: "Closed one" }, "given_back"),
        ],
      }),
      beside(),
    );
    expect(attachments).toEqual([
      expect.objectContaining({ kind: "pull_request", number: 1847, state: "merged", title: "Pin the clock" }),
    ]);
  });

  it("reads a pull request's detail: state, auto-merge, Checks and the names that failed", () => {
    const [pr] = attachmentsOfRecord(
      record("a", {
        attachments: [
          held("pr", "1843", { state: "draft", auto_merge: "true", checks: "failed", failing: "lint, store_test", title: "T", branch: "b", address: "https://x/pull/1843" }),
        ],
      }),
      beside(),
    );
    expect(pr).toEqual({
      kind: "pull_request",
      number: 1843,
      title: "T",
      branch: "b",
      address: "https://x/pull/1843",
      state: "draft",
      auto: true,
      checks: { state: "failed", failing: "lint, store_test" },
    });
  });

  it("reads a pull request nobody has read yet as open, with its Checks pending", () => {
    const [pr] = attachmentsOfRecord(record("a", { attachments: [held("pr", "9")] }), beside());
    expect(pr).toMatchObject({ number: 9, state: "open", checks: { state: "pending" }, title: "", address: "" });
  });

  it("reads a Job off the Board, and marks one the session was handed to look at", () => {
    const board = [job("escalated", { id: "J1", handle: "55-cap-the-backoff", title: "Cap the backoff", branch: "fix/55" })];
    const attachments = attachmentsOfRecord(
      record("a", { attachments: [held("job", "J1", { looking: "true" }), held("job", "J-forgotten")] }),
      beside(board),
    );
    expect(attachments).toEqual([{ kind: "job", id: "J1", number: 55, title: "Cap the backoff", state: "escalated", branch: "fix/55", looking: true }]);
  });

  it("says which Job a slot and a branch came with, where a take over handed them to the session", () => {
    const board = [job("piloted", { id: "J1", handle: "55-cap-the-backoff", title: "Cap the backoff", branch: "fix/55" })];
    const attachments = attachmentsOfRecord(
      record("a", { attachments: [held("slot", "3", { handed: "job J1" }), held("branch", "fix/55", { handed: "job J1" }), held("slot", "4", { handed: "job GONE" })] }),
      beside(board),
    );
    expect(attachments).toContainEqual({ kind: "slot", slot: 3, handed: { job: 55 } });
    expect(attachments).toContainEqual({ kind: "branch", name: "fix/55", slot: 3, handed: { job: 55 } });
    expect(attachments).toContainEqual({ kind: "slot", slot: 4 });
  });

  it("marks a Job a person attested, and a Job piloted from another session", () => {
    const piloted = { reason: "take_over", since: AT };
    const board = [
      job("completed_success", { id: "J1", handle: "55-a", piloted: { ...piloted, session_id: "a", exit: "attested", ended_at: AT } }),
      job("completed_success", { id: "J2", handle: "56-b", piloted: { ...piloted, session_id: "a", exit: "submitted", ended_at: AT } }),
      job("piloted", { id: "J3", handle: "57-c", piloted: { ...piloted, session_id: "a" } }),
      job("piloted", { id: "J4", handle: "58-d", piloted: { ...piloted, session_id: "other" } }),
    ];
    const attachments = attachmentsOfRecord(record("a", { attachments: ["J1", "J2", "J3", "J4"].map((id) => held("job", id)) }), beside(board));
    expect(attachments.map((one) => (one.kind === "job" ? [one.number, one.attested === true, one.pilotedElsewhere === true] : []))).toEqual([
      [55, true, false],
      [56, false, false],
      [57, false, false],
      [58, false, true],
    ]);
  });

  it("takes a Job's number from its handle and its state from its status", () => {
    expect(numberOf({ handle: "52-the-retry-loop" })).toBe(52);
    expect(numberOf({ handle: "no-number" })).toBe(0);
    expect(["queued", "running", "awaiting_review", "completed_success", "escalated", "piloted", "killed"].map(jobStateOf)).toEqual([
      "running",
      "running",
      "review",
      "landed",
      "escalated",
      "piloted",
      "superseded",
    ]);
  });

  it("keeps a subagent's report and says whether it is still running", () => {
    const attachments = attachmentsOfRecord(
      record("a", { attachments: [held("subagent", "x1", { task: "Read the CI history" }), held("subagent", "x2", { task: "Find clocks", report: "None." }, "spent")] }),
      beside(),
    );
    expect(attachments).toEqual([
      { kind: "subagent", id: "x1", task: "Read the CI history", state: "running" },
      { kind: "subagent", id: "x2", task: "Find clocks", state: "done", report: "None." },
    ]);
  });

  it("draws no row for a kind it has no screen for", () => {
    expect(attachmentsOfRecord(record("a", { attachments: [held("need", "docs/x.md"), held("message", "to:s-1234")] }), beside())).toEqual([]);
  });
});

describe("the thread", () => {
  const rows: SessionRow[] = [
    { kind: "message", id: "1", at: AT, from: { kind: "you" }, text: "hi", files: [{ id: "f1", name: "shot.png", media_type: "image/png" }, { id: "f2", name: "n.txt", media_type: "text/plain" }] },
    { kind: "tool", id: "2", at: AT, text: "Bash git status" },
    { kind: "lease", id: "3", at: AT, slot: 3, branch: "fix/x" },
    { kind: "message", id: "4", at: AT, from: { kind: "session", id: "s-1", title: "Other" }, text: "your branch broke main" },
    {
      kind: "ask",
      id: "5",
      at: AT,
      state: "allowed_once",
      ask: { call: "c", manifest_id: "armada", asked_at: AT, tool: "Bash", detail: "git push", truncated: false, rule: "Bash(git push:*)", offers: ["allow_once"], holding_for_seconds: 60 },
    },
    {
      kind: "ask",
      id: "6",
      at: AT,
      state: "waiting",
      ask: { call: "c2", manifest_id: "armada", asked_at: AT, tool: "Bash", detail: "git push -f", truncated: false, rule: "r", offers: ["refuse"], holding_for_seconds: 60 },
    },
  ];

  it("draws a picture where one has been read and a chip where it has not", () => {
    const drawn = rowsOfThread("a", rows, (_session, file) => (file === "f1" ? "blob:one" : undefined));
    expect(drawn[0]).toMatchObject({ kind: "message", files: [{ id: "f1", name: "shot.png", src: "blob:one" }, { id: "f2", name: "n.txt" }] });
    expect(JSON.stringify(drawn[0]).match(/"src"/g)).toHaveLength(1);
  });

  it("keeps a command and a compaction as the rows they are, never a message", () => {
    const drawn = rowsOfThread(
      "a",
      [
        { kind: "command", id: "c", at: AT, text: "/reload-plugins" },
        { kind: "compaction", id: "k", at: AT, text: "This session is being continued" },
      ],
      () => undefined,
    );
    expect(drawn).toMatchObject([{ kind: "command", text: "/reload-plugins" }, { kind: "compaction", text: "This session is being continued" }]);
  });

  it("names another session's message by sender, and keeps the first write as a row", () => {
    const drawn = rowsOfThread("a", rows, () => undefined);
    expect(drawn[2]).toMatchObject({ kind: "lease", slot: 3, branch: "fix/x" });
    expect(drawn[3]).toMatchObject({ from: { kind: "session", id: "s-1", title: "Other" } });
  });

  it("draws the handoff first in a piloted thread, with what Fleet compared and the Drone's list taken apart", () => {
    const handoff: SessionRow = {
      kind: "handoff",
      id: "h",
      at: AT,
      job_id: "J1",
      number: 55,
      title: "Cap the backoff",
      reason: "take_over",
      slot: 3,
      branch: "fix/55",
      step: { id: "verify", label: "Verify the fix" },
      attempts: 3,
      refusals: ["No test covers the cap"],
      plan: { declared: true, outside: ["a/loop.rs"], unwritten: ["a/tests.rs"] },
      narrative: { trying_to: "Cap it", blocked_by: "The lint", tried: "- added a loop\n2. added a sleep\n\n  ran the Check" },
    };
    const [drawn] = rowsOfThread("a", [handoff], () => undefined);
    expect(drawn).toEqual({
      id: "h",
      at: expect.any(String),
      kind: "handoff",
      job: { number: 55, title: "Cap the backoff" },
      slot: 3,
      branch: "fix/55",
      step: { id: "verify", label: "Verify the fix" },
      attempts: 3,
      refusals: ["No test covers the cap"],
      plan: { outside: ["a/loop.rs"], unwritten: ["a/tests.rs"] },
      narrative: { trying_to: "Cap it", blocked_by: "The lint", tried: ["added a loop", "added a sleep", "ran the Check"] },
    });
  });

  it("leaves out of a handoff what Fleet had none of: no step, no refusals, no plan, no narrative", () => {
    const [drawn] = rowsOfThread("a", [{ kind: "handoff", id: "h", at: AT, job_id: "J1", number: 55, title: "T", reason: "take_over", slot: 3, branch: "b", attempts: 0, plan: { declared: false } }], () => undefined);
    expect(drawn).toMatchObject({ kind: "handoff", refusals: [], plan: { outside: [], unwritten: [] } });
    expect(drawn).not.toHaveProperty("step");
    expect(drawn).not.toHaveProperty("narrative");
  });

  it("says what an answered ask was decided and leaves a waiting one to the card under the thread", () => {
    const drawn = rowsOfThread("a", rows, () => undefined);
    expect(drawn.map((one) => one.id)).toEqual(["1", "2", "3", "4", "5"]);
    expect(drawn[4]).toMatchObject({ kind: "tool", text: "git push: allowed once" });
  });
});

describe("a stored title", () => {
  it("loses the harness's markup, and a title that is only markup is no title", () => {
    expect(cleanTitle('<agent-message from="a45d14071172cd311">Review the ledger change')).toBe("Review the ledger change");
    expect(cleanTitle("<system-reminder>noise</system-reminder>Fix it")).toBe("Fix it");
    expect(cleanTitle('<agent-message from="a45d14071172cd311">')).toBeUndefined();
    expect(sessionOfRecord(record("a", { title: "<agent-message from=\"x\">" }), undefined, beside()).title).toBeUndefined();
    expect(sessionOfRecord(record("a", { title: "<agent-message from=\"x\">Fix it" }), undefined, beside()).title).toBe("Fix it");
  });
});

describe("a session", () => {
  it("carries its turn, ask, tuning and address", () => {
    const session = sessionOfRecord(
      record("01ABCDEFGHJKMNPQRSTVWXYZ00", {
        title: "Fix it",
        last_turn_at: "2026-10-07T13:50:09Z",
        hosted: {
          turn: { state: "working", woken_by: { id: "s-2", title: "Other" } },
          model: "opus",
          effort: "high",
          mode: "plan",
          running: true,
          asked: { call: "c", manifest_id: "armada", asked_at: AT, tool: "Bash", detail: "git push", truncated: false, rule: "r", offers: ["allow_once", "refuse"], holding_for_seconds: 60 },
        },
      }),
      undefined,
      beside(),
    );
    expect(session).toMatchObject({
      address: "s-01ABCDEF",
      title: "Fix it",
      turn: { state: "working", wokenBy: { id: "s-2", title: "Other" } },
      asked: { command: "git push", call: "c", offers: ["allow_once", "refuse"] },
      model: "opus",
      effort: "high",
      mode: "plan",
    });
    expect(session.lastTurn).toMatch(/^\d\d:\d\d$/);
  });

  it("is a blank one until it has been written to, titled or finished a turn, even before its thread is opened", () => {
    expect(sessionOfRecord(record("a"), undefined, beside()).blank).toBe(true);
    expect(sessionOfRecord(record("a", { title: "Named" }), undefined, beside()).blank).toBe(false);
    expect(sessionOfRecord(record("a", { attachments: [held("slot", "1")] }), undefined, beside()).blank).toBe(false);
  });

  it("marks a terminal session, which has no thread to write in", () => {
    const { hosted: _hosted, ...rest } = record("t", { origin: "terminal" });
    expect(sessionOfRecord(rest, undefined, beside())).toMatchObject({ terminal: true });
  });

  it("lists the sessions still open and the ones that ended, and who owns a branch is the one holding it across repositories", () => {
    const sessions = sessionsOfRecords(
      [
        record("a", { title: "Mine", attachments: [held("branch", "fix/x"), held("slot", "3")] }),
        record("b", { title: "Elsewhere", manifest_id: "storefront", attachments: [held("branch", "feat/y", {}, "standing", "storefront"), held("slot", "1", {}, "standing", "storefront")] }),
        record("c", { title: "Closed", state: "ended", attachments: [held("branch", "old")] }),
      ],
      {},
      () => beside(),
    );
    expect(sessions.map((one) => one.id)).toEqual(["a", "b", "c"]);
    expect(sessions[2]).toMatchObject({ dead: "ended", attachments: [] });
    expect(ownerOf(sessions, { kind: "branch", name: "feat/y" })?.id).toBe("b");
    expect(ownerOf(sessions, { kind: "branch", name: "old" })).toBeUndefined();
    expect(sessionsMatching(sessions, "feat/y").map((hit) => hit.session.id)).toEqual(["b"]);
  });

  it("marks a session that ended more than a week ago, and leaves open ones and recent ones unmarked", () => {
    const now = Date.parse(AT);
    const day = 24 * 60 * 60 * 1000;
    const ago = (days: number) => new Date(now - days * day).toISOString();
    const sessions = sessionsOfRecords(
      [
        record("open", { last_seen_at: ago(30) }),
        record("recent", { state: "ended", last_seen_at: ago(ENDED_LISTED_DAYS - 1) }),
        record("old", { state: "ended", last_seen_at: ago(ENDED_LISTED_DAYS + 1) }),
      ],
      {},
      () => beside(),
      now,
    );
    expect(sessions.filter((one) => one.older === true).map((one) => one.id)).toEqual(["old"]);
  });
});

describe("a session nothing can be said to", () => {
  const { hosted: _hosted, ...asTerminal } = record("t", { origin: "terminal" });

  it("is a terminal session whose mod has not asked lately, and not one whose mod has", () => {
    expect(sessionOfRecord(asTerminal, undefined, beside()).dead).toBe("quiet");
    expect(sessionOfRecord({ ...asTerminal, terminal: { listening: true } }, undefined, beside()).dead).toBeUndefined();
  });

  it("stays quiet however long ago it was seen, and is not marked older, which only an ended session is", () => {
    const old = "2026-01-01T00:00:00Z";
    const [session] = sessionsOfRecords([{ ...asTerminal, last_seen_at: old }], {}, () => beside(), Date.parse(AT));
    expect(session).toMatchObject({ dead: "quiet" });
    expect(session?.older).toBeUndefined();
  });

  it("is never a hosted session that is open, whatever its process is doing", () => {
    expect(sessionOfRecord(record("h"), undefined, beside()).dead).toBeUndefined();
  });

  it("is any session that ended, and one that ended with a terminal still listening is ended", () => {
    expect(sessionOfRecord(record("h", { state: "ended" }), undefined, beside()).dead).toBe("ended");
    expect(sessionOfRecord({ ...asTerminal, state: "ended", terminal: { listening: true } }, undefined, beside()).dead).toBe("ended");
  });

  it("keeps the link to its forks and nothing else it held", () => {
    const ended = record("old", {
      state: "ended",
      attachments: [held("branch", "fix/x"), held("slot", "3"), held("forked_to", "new", {}, "spent")],
    });
    expect(sessionOfRecord(ended, undefined, beside()).attachments).toEqual([{ kind: "forked_to", id: "new" }]);
    const fork = record("new", { attachments: [held("forked_from", "old", {}, "spent")] });
    expect(sessionOfRecord(fork, undefined, beside()).attachments).toEqual([{ kind: "forked_from", id: "old" }]);
  });
});
