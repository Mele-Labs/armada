import { describe, expect, it } from "vitest";

import { NOTICES } from "@armada/protocol";
import type { Attachment, AttachmentHistory, HelmCallInFlight, SessionRecord, SessionRow } from "@armada/protocol";
import { ENDED_LISTED_DAYS, SessionMemo, activityOf, attachmentsOfRecord, cleanTitle, lapsedAnswerText, lapsedOf, rowsOfThread, sceneOfWire, sessionOfRecord, sessionsOfRecords, sketchCallOf, trailOfRecord, versionsOfHistory } from "./sessions-wire";
import type { Beside } from "./sessions-wire";
import { ownerOf, sessionsMatching } from "./draft/sessions";
import { share } from "./share";

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

const beside = (): Beside => ({ picture: () => undefined, sketches: [], pending: [] });

describe("the ledger", () => {
  it("pairs a branch with the slot the session holds in the same repository", () => {
    const attachments = attachmentsOfRecord(
      record("a", { attachments: [held("slot", "2", {}, "standing", "storefront"), held("slot", "3"), held("branch", "fix/flaky-store")] }),
      beside(),
    );
    expect(attachments).toContainEqual({ kind: "branch", name: "fix/flaky-store", slot: 3 });
    expect(attachments).toContainEqual({ kind: "slot", slot: 3 });
  });

  it("keeps a page it showed as an artifact, and a message channel it holds adds no row of its own", () => {
    const attachments = attachmentsOfRecord(
      record("a", {
        attachments: [
          held("slot", "9"),
          held("branch", "bridge/queue-for-later"),
          held("artifact", "http://localhost:40108/?walk=queue", { form: "window", title: "Queue for later walk" }),
          held("message", "from:coordinator", { count: "15", direction: "received" }),
        ],
      }),
      beside(),
    );
    expect(attachments).toContainEqual({ kind: "artifact", form: "window", id: "http://localhost:40108/?walk=queue", title: "Queue for later walk" });
    expect(attachments.map((one) => one.kind).sort()).toEqual(["artifact", "branch", "slot"]);
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

  it("reads names left in `failing` beside a pending or passed verdict as a check red outside the gate", () => {
    const [running, after] = attachmentsOfRecord(
      record("a", {
        attachments: [held("pr", "1", { checks: "pending", failing: "app_smoke_shard (2)" }), held("pr", "2", { checks: "passed", failing: "app_smoke_shard (2)" })],
      }),
      beside(),
    );
    expect(running).toMatchObject({ checks: { state: "pending", red: "app_smoke_shard (2)" } });
    expect(after).toMatchObject({ checks: { state: "passed", red: "app_smoke_shard (2)" } });
  });

  it("reads a pull request nobody has read yet as open, with its Checks pending", () => {
    const [pr] = attachmentsOfRecord(record("a", { attachments: [held("pr", "9")] }), beside());
    expect(pr).toMatchObject({ number: 9, state: "open", checks: { state: "pending" }, title: "", address: "" });
  });

  it("reads an issue row: its number, title, address and state", () => {
    const [one] = attachmentsOfRecord(record("a", { attachments: [held("issue", "2212", { title: "Helm loses the repository", state: "closed", address: "https://x/issues/2212" })] }), beside());
    expect(one).toEqual({ kind: "issue", number: "2212", title: "Helm loses the repository", address: "https://x/issues/2212", state: "closed" });
  });

  it("reads an issue the pull reading has not filled yet as open, with no title and no address", () => {
    const [bare, empty] = attachmentsOfRecord(record("a", { attachments: [held("issue", "7", {}), { ...held("issue", "8"), detail: undefined }] }), beside());
    expect(bare).toEqual({ kind: "issue", number: "7", title: "", address: "", state: "open" });
    expect(empty).toEqual({ kind: "issue", number: "8", title: "", address: "", state: "open" });
  });

  it("reads a tracker's issue by its key, with the pull requests its row names", () => {
    const [one] = attachmentsOfRecord(record("a", { attachments: [held("issue", "ABC-12", { title: "Checkout total rounds twice", pull_requests: "7,31" })] }), beside());
    expect(one).toEqual({ kind: "issue", number: "ABC-12", title: "Checkout total rounds twice", address: "", state: "open", pull_requests: [7, 31] });
  });

  it("reads the issues a pull request closes, in the order the wire gives them", () => {
    const [pr] = attachmentsOfRecord(record("a", { attachments: [held("pr", "2256", { closes: "2212,2259" })] }), beside());
    expect(pr).toMatchObject({ kind: "pull_request", closes: [2212, 2259] });
  });

  it("leaves `closes` off a pull request that names no issue", () => {
    const [none, blank] = attachmentsOfRecord(record("a", { attachments: [held("pr", "1"), held("pr", "2", { closes: "" })] }), beside());
    expect(none).not.toHaveProperty("closes");
    expect(blank).not.toHaveProperty("closes");
  });

  it("reads the branch a pull request is based on", () => {
    const [pr] = attachmentsOfRecord(record("a", { attachments: [held("pr", "2", { base: "feature/one" })] }), beside());
    expect(pr).toMatchObject({ kind: "pull_request", base: "feature/one" });
  });

  it("leaves `base` off a pull request that names none", () => {
    const [none, blank] = attachmentsOfRecord(record("a", { attachments: [held("pr", "1"), held("pr", "2", { base: "  " })] }), beside());
    expect(none).not.toHaveProperty("base");
    expect(blank).not.toHaveProperty("base");
  });

  it("leaves out a ledger row of a kind this build has no screen for", () => {
    expect(attachmentsOfRecord(record("a", { attachments: [held("job", "J1")] }), beside())).toEqual([]);
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

  it("keeps the call that started a subagent and the type it was started as", () => {
    const attachments = attachmentsOfRecord(record("a", { attachments: [held("subagent", "a4c6", { task: "Mock walk", type: "bridge-engineer", call: "toolu_1" })] }), beside());
    expect(attachments).toEqual([{ kind: "subagent", id: "a4c6", task: "Mock walk", state: "running", agentType: "bridge-engineer", call: "toolu_1" }]);
  });

  it("reads a background task as running, done, failed or stopped, and keeps one given back", () => {
    const detail = { form: "command", task: "Build it", command: "pnpm build", output: "/tmp/t/b.output" };
    const attachments = attachmentsOfRecord(
      record("a", {
        attachments: [
          held("background", "b1", detail),
          held("background", "b2", { ...detail, status: "completed", exit: "0" }, "spent"),
          held("background", "b3", { ...detail, status: "failed", exit: "2" }, "spent"),
          held("background", "b4", { ...detail, status: "killed" }, "spent"),
          held("background", "b5", { form: "monitor", task: "Watch CI", command: "gh run watch 9" }, "given_back"),
          held("background", "b6", { form: "other" }),
        ],
      }),
      beside(),
    );
    const base = { kind: "background", form: "command", task: "Build it", command: "pnpm build", started: AT, output: "/tmp/t/b.output" };
    expect(attachments).toEqual([
      { ...base, id: "b1", state: "running" },
      { ...base, id: "b2", state: "done", ended: AT, exit: 0 },
      { ...base, id: "b3", state: "failed", ended: AT, exit: 2 },
      { ...base, id: "b4", state: "stopped", ended: AT },
      { kind: "background", id: "b5", form: "monitor", task: "Watch CI", command: "gh run watch 9", started: AT, state: "stopped", ended: AT },
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

  it("draws words Helm sent on as a row of their own, with where they went and the repository's name", () => {
    const sent: SessionRow = {
      kind: "sent",
      id: "7",
      at: AT,
      text: "wait for the merge",
      target: { kind: "session", id: "s-9", title: "Sessions list views", manifest_id: "storefront" },
      files: [{ id: "f9", name: "shot.png", media_type: "image/png" }],
    };
    const [drawn] = rowsOfThread("a", [sent], () => "blob:nine", (manifest) => (manifest === "storefront" ? "Storefront" : (manifest ?? "")));
    expect(drawn).toMatchObject({
      kind: "sent",
      text: "wait for the merge",
      target: { kind: "session", id: "s-9", repository: "Storefront" },
      files: [{ id: "f9", name: "shot.png", src: "blob:nine" }],
    });
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

  it("keeps a disputed row with its notice, and a review only where one was started", () => {
    const drawn = rowsOfThread(
      "a",
      [
        { kind: "disputed", id: "d1", at: AT, notice: "n1", why: "Reading agents run on sonnet.", spotted: "called", review: "r-1" },
        { kind: "disputed", id: "d2", at: AT, notice: "n2", why: "Reading agents run on sonnet.", spotted: "read" },
      ],
      () => undefined,
    );
    expect(drawn).toEqual([
      { id: "d1", at: AT, kind: "disputed", notice: "n1", why: "Reading agents run on sonnet.", spotted: "called", review: "r-1" },
      { id: "d2", at: AT, kind: "disputed", notice: "n2", why: "Reading agents run on sonnet.", spotted: "read" },
    ]);
  });

  it("marks an answered ask only where the phone answered it", () => {
    const ask = { call: "c", manifest_id: "armada", asked_at: AT, tool: "Bash", detail: "rm -rf build", truncated: false, rule: "Bash(rm:*)", offers: [], holding_for_seconds: 600 };
    const drawn = rowsOfThread(
      "a",
      [
        { kind: "ask", id: "p", at: AT, ask, state: "allowed_once", via: "phone" },
        { kind: "ask", id: "b", at: AT, ask, state: "allowed_once", via: "bridge" },
        { kind: "ask", id: "n", at: AT, ask, state: "refused" },
      ] as SessionRow[],
      () => undefined,
    );
    expect(drawn.map((row) => ("fromPhone" in row ? row.fromPhone : undefined))).toEqual([true, undefined, undefined]);
  });

  it("carries each row's instant as it came, for the component to word in the owner's format", () => {
    const drawn = rowsOfThread("a", [{ kind: "window", id: "w", at: AT, title: "Findings", url: "http://localhost:5173/" }], () => undefined);
    expect(drawn[0]?.at).toBe(AT);
  });

  it("keeps an armada check the agent ran as a row with its run, where it stands and where Fleet read it failed", () => {
    const blame = { path: "apps/desktop/src/layout.ts", line: 142 };
    const drawn = rowsOfThread("a", [{ kind: "check", id: "check-7", at: AT, name: "components_test", run: 7, state: "failed", blame }], () => undefined);
    expect(drawn).toMatchObject([{ kind: "check", name: "components_test", run: 7, state: "failed", blame }]);
  });

  it("keeps a run that ended as a row with its exit and the lines it was sent, and a stopped one with no code", () => {
    const drawn = rowsOfThread(
      "a",
      [
        { kind: "ran", id: "r1", at: AT, command: "make", code: 2, tail: "error: no rule" },
        { kind: "ran", id: "r2", at: AT, command: "sleep 30", tail: "" },
      ],
      () => undefined,
    );
    expect(drawn).toMatchObject([
      { kind: "ran", command: "make", code: 2, tail: "error: no rule" },
      { kind: "ran", command: "sleep 30", code: null, tail: "" },
    ]);
  });

  it("keeps a request whole to send again, and a failed one's error", () => {
    const drawn = rowsOfThread(
      "a",
      [
        {
          kind: "requested",
          id: "q1",
          at: AT,
          by: "owner",
          method: "POST",
          url: "https://api.example.test/v1/orders",
          environment: "staging",
          headers: [{ name: "Content-Type", value: "application/json" }],
          body: "{}",
          status: 201,
          ms: 320,
          response: "{\"id\":7}",
        },
        { kind: "requested", id: "q2", at: AT, by: "agent", method: "GET", url: "http://localhost:9/", ms: 3, error: "connection refused", response: "" },
      ],
      () => undefined,
    );
    expect(drawn).toMatchObject([
      { kind: "requested", by: "owner", method: "POST", environment: "staging", headers: [{ name: "Content-Type", value: "application/json" }], body: "{}", status: 201, ms: 320, response: "{\"id\":7}", truncated: false },
      { kind: "requested", by: "agent", status: null, error: "connection refused", headers: [], truncated: false },
    ]);
  });

  it("draws a request the agent saved as the collection and request to open in Requests", () => {
    const drawn = rowsOfThread("a", [{ kind: "saved", id: "v1", at: AT, name: "Create order", collection: "Store API", collection_id: "store-api", request: "create-order" }], () => undefined);
    expect(drawn).toEqual([{ id: "v1", at: AT, kind: "saved", name: "Create order", collection: "Store API", collectionId: "store-api", request: "create-order" }]);
  });

  it("keeps a draft whole as a draft row, open and once settled", () => {
    const proposed = { kind: "draft", id: "d", at: AT, title: "Description for the queue pull request", text: "A Job waits for a free slot.", before: "Adds a queue." } as const;
    expect(rowsOfThread("a", [proposed], () => undefined)).toEqual([{ id: "d", at: AT, kind: "draft", title: "Description for the queue pull request", text: "A Job waits for a free slot.", before: "Adds a queue." }]);
    const settled = { ...proposed, settled: { how: "sent_back", words: "Say what a restart keeps.", at: AT } } as const;
    expect(rowsOfThread("a", [settled], () => undefined)).toEqual([{ ...proposed, settled: { how: "sent_back", words: "Say what a restart keeps.", at: AT } }]);
  });

  it("keeps a question asked in the window as a line, with how it was answered once it is", () => {
    const asked = {
      kind: "window_ask",
      id: "q",
      at: AT,
      question: "Beside or under?",
      place: { url: "http://localhost:5173/", selector: "main > h1" },
      window: "http://localhost:5173/",
    } as const;
    expect(rowsOfThread("a", [asked], () => undefined)).toMatchObject([{ kind: "tool", text: "Asked in the window: Beside or under?" }]);
    expect(rowsOfThread("a", [{ ...asked, answer: "Under\nand smaller" }], () => undefined)).toMatchObject([
      { kind: "tool", text: "Asked in the window: Beside or under? — Under · and smaller" },
    ]);
  });

  it("keeps a window the Session showed as a row with its address, and its ledger entry as a window", () => {
    const drawn = rowsOfThread("a", [{ kind: "window", id: "w", at: AT, title: "Findings", url: "http://localhost:5173/" }], () => undefined);
    expect(drawn).toMatchObject([{ kind: "window", title: "Findings", url: "http://localhost:5173/" }]);
    const shown = { ...held("artifact", "http://localhost:5173/"), detail: { form: "window", title: "Findings" } };
    expect(attachmentsOfRecord(record("a", { attachments: [shown] }), beside())).toEqual([
      { kind: "artifact", form: "window", id: "http://localhost:5173/", title: "Findings" },
    ]);
  });

  it("tells a file the Session wrote from one it only showed, whether or not it also showed the first", () => {
    const file = (detail: Record<string, string>) => ({ ...held("artifact", "/work/notes.md"), detail: { form: "file", title: "notes.md", ...detail } });
    const SHOWN = { by: "agent", shown: "2026-10-19T14:44:20.113Z" };
    const kept = (detail: Record<string, string>) => attachmentsOfRecord(record("a", { attachments: [file(detail)] }), beside());
    expect(kept({})).toEqual([{ kind: "artifact", form: "file", id: "/work/notes.md", title: "notes.md" }]);
    expect(kept(SHOWN)).toEqual([{ kind: "artifact", form: "file", id: "/work/notes.md", title: "notes.md", shown: SHOWN.shown, by: "agent" }]);
    expect(kept({ ...SHOWN, written: "true" })).toEqual([{ kind: "artifact", form: "file", id: "/work/notes.md", title: "notes.md", shown: SHOWN.shown, by: "agent", written: true }]);
  });

  it("keeps a document the Job wrote with its revisions, and the row alone where they cannot be read", () => {
    const page = (detail: Record<string, string>) => ({ ...held("artifact", "/work/.armada/documents/findings.html"), detail: { form: "document", title: "Findings", ...detail } });
    const kept = (detail: Record<string, string>) => attachmentsOfRecord(record("a", { attachments: [page(detail)] }), beside());
    const row = { kind: "artifact", form: "document", id: "/work/.armada/documents/findings.html", title: "Findings" };
    expect(kept({ revisions: JSON.stringify(["/store/r1.html", "/store/r2.html"]) })).toEqual([{ ...row, revisions: ["/store/r1.html", "/store/r2.html"] }]);
    expect(kept({ revisions: "[not json" })).toEqual([row]);
    expect(kept({ revisions: JSON.stringify({ r1: "/store/r1.html" }) })).toEqual([row]);
    expect(kept({ revisions: JSON.stringify(["/store/r1.html", 2]) })).toEqual([row]);
    expect(kept({})).toEqual([row]);
  });

  it("carries the notice a Fleet message was sent for, and no other row", () => {
    const wire: SessionRow[] = [
      { kind: "message", id: "n1", at: AT, from: { kind: "session", id: "fleet", title: "Fleet" }, text: "slot-4 is 23 commits behind master.", notice: "behind_base" },
      { kind: "message", id: "n2", at: AT, from: { kind: "session", id: "fleet", title: "Fleet" }, text: "Context is 180k tokens." },
    ];
    const drawn = rowsOfThread("a", wire, () => undefined);
    expect(drawn[0]).toMatchObject({ notice: "behind_base" });
    expect(drawn[1]).not.toHaveProperty("notice");
  });

  it("carries every notice word Bridge can mute, and drops one it does not know", () => {
    const fleet = { kind: "session", id: "fleet", title: "Fleet" } as const;
    const wire = [...NOTICES.map((notice, at) => ({ kind: "message", id: `n${at}`, at: AT, from: fleet, text: notice, notice })), { kind: "message", id: "x", at: AT, from: fleet, text: "x", notice: "from_a_newer_fleet" }] as SessionRow[];
    const drawn = rowsOfThread("a", wire, () => undefined);
    expect(drawn.slice(0, NOTICES.length).map((one) => (one as { notice?: string }).notice)).toEqual([...NOTICES]);
    expect(drawn.at(-1)).not.toHaveProperty("notice");
  });

  it("names another session's message by sender, and keeps the first write as a row", () => {
    const drawn = rowsOfThread("a", rows, () => undefined);
    expect(drawn[2]).toMatchObject({ kind: "lease", slot: 3, branch: "fix/x" });
    expect(drawn[3]).toMatchObject({ from: { kind: "session", id: "s-1", title: "Other" } });
  });

  it("keeps the slot given back as a row of its own", () => {
    const drawn = rowsOfThread("a", [{ kind: "release", id: "9", at: AT, slot: 3, branch: "fix/x" }], () => undefined);
    expect(drawn[0]).toMatchObject({ kind: "release", slot: 3, branch: "fix/x" });
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

describe("a session's trail", () => {
  const at = (row: Attachment, since: string, changed = since): Attachment => ({ ...row, since, changed_at: changed });

  it("is when it started and nothing else where it held no slot and opened no pull request", () => {
    expect(trailOfRecord(record("a", { started_at: "2026-10-13T08:00:00Z" }))).toEqual({ started: "2026-10-13T08:00:00Z" });
    expect(sessionOfRecord(record("a"), undefined, beside()).trail).toEqual({ started: AT });
  });

  it("reads how the Job was started, naming the Job that started it", () => {
    const started = { workflow: "Feature", delivery: "open_pr" as const, model: "opus", effort: "high", mode: "plan" as const, by: { id: "s-parent", title: "Plan the lease" } };
    expect(sessionOfRecord(record("a", { started }), undefined, beside()).started).toEqual({ ...started, by: { job: "s-parent", title: "Plan the lease" } });
    expect(sessionOfRecord(record("a"), undefined, beside()).started).toBeUndefined();
  });

  it("keeps each slot it held, a given-back one with when it let go, which the ledger leaves out", () => {
    const wire = record("a", {
      started_at: "2026-10-13T08:00:00Z",
      attachments: [
        at(held("slot", "5"), "2026-10-13T09:30:00Z"),
        at(held("slot", "2", {}, "given_back"), "2026-10-13T08:20:00Z", "2026-10-13T09:00:00Z"),
      ],
    });
    expect(trailOfRecord(wire)).toEqual({
      started: "2026-10-13T08:00:00Z",
      bays: [
        { slot: 2, from: "2026-10-13T08:20:00Z", until: "2026-10-13T09:00:00Z" },
        { slot: 5, from: "2026-10-13T09:30:00Z" },
      ],
    });
    expect(attachmentsOfRecord(wire, beside())).toEqual([{ kind: "slot", slot: 5 }]);
  });

  it("keeps when each pull request was recorded, by number, one let go too", () => {
    const wire = record("a", {
      attachments: [at(held("pr", "1847", { title: "T" }, "spent"), "2026-10-13T10:00:00Z"), at(held("pr", "1850", {}, "given_back"), "2026-10-13T10:05:00Z"), held("branch", "fix/x")],
    });
    expect(trailOfRecord(wire).opened).toEqual({ 1847: "2026-10-13T10:00:00Z", 1850: "2026-10-13T10:05:00Z" });
  });

  it("stays on a session that ended", () => {
    const ended = record("a", { state: "ended", attachments: [at(held("slot", "2", {}, "given_back"), "2026-10-13T08:20:00Z", "2026-10-13T09:00:00Z")] });
    expect(sessionOfRecord(ended, undefined, beside()).trail?.bays).toEqual([{ slot: 2, from: "2026-10-13T08:20:00Z", until: "2026-10-13T09:00:00Z" }]);
  });
});

describe("a session", () => {
  it("carries Helm's mark from the record", () => {
    expect(sessionOfRecord(record("01ABCDEFGHJKMNPQRSTVWXYZ01", { helm: true }), undefined, beside())).toMatchObject({ helm: true });
    expect(sessionOfRecord(record("01ABCDEFGHJKMNPQRSTVWXYZ02"), undefined, beside()).helm).toBeUndefined();
  });

  it("carries the Mod mark from the record", () => {
    expect(sessionOfRecord(record("01ABCDEFGHJKMNPQRSTVWXYZ01", { mod: true }), undefined, beside())).toMatchObject({ mod: true });
    expect(sessionOfRecord(record("01ABCDEFGHJKMNPQRSTVWXYZ02"), undefined, beside()).mod).toBeUndefined();
  });

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
    expect(session.lastTurnAt).toBe("2026-10-07T13:50:09Z");
  });

  it("carries the model its process resolved apart from the one picked, and neither where none was said", () => {
    const hosted = (change: object) => ({ turn: { state: "idle" as const }, mode: "auto" as const, running: true, ...change });
    const of = (change: object) => sessionOfRecord(record("01ABCDEFGHJKMNPQRSTVWXYZ02", { hosted: hosted(change) }), undefined, beside());

    expect(of({ model: "opus", resolved_model: "vendor-opus-5-5" })).toMatchObject({ model: "opus", resolvedModel: "vendor-opus-5-5" });
    expect(of({ resolved_model: "vendor-opus-5-5" }).model).toBeUndefined();
    const unsaid = of({});
    expect(unsaid.model).toBeUndefined();
    expect(unsaid.resolvedModel).toBeUndefined();
  });

  it("marks a session paused from hosted.paused, and none otherwise", () => {
    const hosted = (change: object) => ({ turn: { state: "idle" as const }, mode: "auto" as const, running: false, ...change });
    const of = (change: object) => sessionOfRecord(record("01ABCDEFGHJKMNPQRSTVWXYZ03", { hosted: hosted(change) }), undefined, beside());

    expect(of({ paused: { at: "2026-10-17T10:00:00Z", resuming: false } }).paused).toBe(true);
    expect(of({ paused: { at: "2026-10-17T10:00:00Z", resuming: true } }).paused).toBe(true);
    expect(of({}).paused).toBeUndefined();
  });

  it("marks a session Auto land from auto_land, and none where it is absent or the session ended", () => {
    const hosted = { turn: { state: "idle" as const }, mode: "auto" as const, running: false };
    const of = (change: object) => sessionOfRecord(record("01ABCDEFGHJKMNPQRSTVWXYZ04", { hosted, ...change }), undefined, beside());

    expect(of({ auto_land: true }).autoLand).toBe(true);
    expect(of({}).autoLand).toBeUndefined();
    expect(of({ auto_land: true, state: "ended" }).autoLand).toBeUndefined();
  });

  it("asks for a cut command whole, and for one that fits as its line", () => {
    const whole = `${"echo ok;".repeat(36)} echo the-last-word`;
    const of = (asked: Pick<HelmCallInFlight, "detail" | "truncated" | "length" | "whole">) =>
      sessionOfRecord(
        record("01ABCDEFGHJKMNPQRSTVWXYZ04", {
          hosted: { turn: { state: "working" }, mode: "auto", running: true, asked: { call: "c", manifest_id: "armada", asked_at: AT, tool: "Bash", rule: "r", offers: ["allow_once", "refuse"], holding_for_seconds: 60, ...asked } },
        }),
        undefined,
        beside(),
      ).asked;

    expect(of({ detail: whole.slice(0, 200), truncated: true, length: whole.length, whole })?.command).toBe(whole);
    expect(of({ detail: "git push", truncated: false, length: 8 })?.command).toBe("git push");
  });

  it("carries the agent's questions on its ask", () => {
    const questions = [{ question: "Which size?", header: "Size", multi_select: false, options: [{ label: "S", description: "Small" }] }];
    const session = sessionOfRecord(
      record("01ABCDEFGHJKMNPQRSTVWXYZ01", {
        hosted: {
          turn: { state: "working" },
          mode: "auto",
          running: true,
          asked: { call: "c", manifest_id: "armada", asked_at: AT, tool: "AskUserQuestion", detail: "Which size?", truncated: false, rule: "AskUserQuestion", offers: ["allow_once", "refuse"], holding_for_seconds: 60, questions },
        },
      }),
      undefined,
      beside(),
    );
    expect(session.asked).toMatchObject({ command: "Which size?", call: "c", questions });
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

  it("keeps what an ended session made, its pull requests and artifacts and the link to its forks, and drops its slot and branch", () => {
    const ended = record("old", {
      state: "ended",
      attachments: [
        held("slot", "3"),
        held("branch", "fix/x"),
        held("pr", "1847", { state: "merged", title: "Pin the clock", checks: "passed" }, "spent"),
        held("pr", "1850", { title: "Open one", checks: "pending" }),
        held("artifact", "https://example.com/a", { form: "page", title: "Findings" }, "spent"),
        held("forked_to", "new", {}, "spent"),
      ],
    });
    expect(sessionOfRecord(ended, undefined, beside()).attachments).toEqual([
      expect.objectContaining({ kind: "pull_request", number: 1847, state: "merged", checks: { state: "passed" } }),
      expect.objectContaining({ kind: "pull_request", number: 1850, state: "open", checks: { state: "pending" } }),
      { kind: "artifact", form: "page", id: "https://example.com/a", title: "Findings" },
      { kind: "forked_to", id: "new" },
    ]);
  });

  it("keeps the link to its forks and nothing else it held", () => {
    const ended = record("old", {
      state: "ended",
      attachments: [held("branch", "fix/x"), held("slot", "3"), held("forked_to", "new", {}, "spent")],
    });
    expect(sessionOfRecord(ended, undefined, beside()).attachments).toEqual([{ kind: "forked_to", id: "new" }]);
    const fork = record("new", { attachments: [held("forked_from", "old", {}, "spent")] });
    expect(sessionOfRecord(fork, undefined, beside()).attachments).toEqual([{ kind: "forked_from", id: "old" }]);
    // A fork at a message carries the row it was taken at, which cuts the old rows shown; `cut` is Fleet's.
    const at = record("new", { attachments: [held("forked_from", "old", { at: "m2", cut: "7d0a52c4" }, "spent")] });
    expect(sessionOfRecord(at, undefined, beside()).attachments).toEqual([{ kind: "forked_from", id: "old", at: "m2" }]);
    const moved = record("new", { attachments: [held("forked_from", "old", { moved: "01OLD", branch: "armada/x" }, "spent")] });
    expect(sessionOfRecord(moved, undefined, beside()).attachments).toEqual([
      { kind: "forked_from", id: "old", moved: "01OLD", branch: "armada/x" },
    ]);
    // The old Session's side of a move names the repository it went to, and carries no branch.
    const from = record("old", { attachments: [held("forked_to", "new", { moved: "01NEW" }, "spent")] });
    expect(sessionOfRecord(from, undefined, beside()).attachments).toEqual([{ kind: "forked_to", id: "new", moved: "01NEW" }]);
    // A move that left no branch carries none.
    const bare = record("new", { attachments: [held("forked_from", "old", { moved: "01OLD" }, "spent")] });
    expect(sessionOfRecord(bare, undefined, beside()).attachments).toEqual([{ kind: "forked_from", id: "old", moved: "01OLD" }]);
  });
});

describe("what the agent keeps for later", () => {
  const later = [{ id: "later-1", sentence: "I will file the slow fixture load.", said_at: AT }];

  it("carries each open follow-up with its id, its words and when it was said", () => {
    expect(sessionOfRecord(record("a", { later }), undefined, beside()).later).toEqual([{ id: "later-1", text: "I will file the slow fixture load.", at: AT }]);
  });

  it("carries a thumbs up Fleet kept as confirmed, and says nothing where there is none", () => {
    const marked = [{ ...later[0]!, confirmed: true }, { id: "later-2", sentence: "Name the loader.", said_at: AT }];
    expect(sessionOfRecord(record("a", { later: marked }), undefined, beside()).later).toEqual([
      { id: "later-1", text: "I will file the slow fixture load.", at: AT, confirmed: true },
      { id: "later-2", text: "Name the loader.", at: AT },
    ]);
  });

  it("carries none for a session with none, or one that ended", () => {
    expect(sessionOfRecord(record("a"), undefined, beside()).later).toBeUndefined();
    expect(sessionOfRecord(record("a", { state: "ended", later }), undefined, beside()).later).toBeUndefined();
  });
});

describe("when the agent said it was done", () => {
  it("carries done_since onto the session as done", () => {
    expect(sessionOfRecord(record("a", { done_since: AT }), undefined, beside()).done).toEqual({ since: AT });
  });

  it("carries none for a session that has not said it, or one that ended", () => {
    expect(sessionOfRecord(record("a"), undefined, beside()).done).toBeUndefined();
    expect(sessionOfRecord(record("a", { state: "ended", done_since: AT }), undefined, beside()).done).toBeUndefined();
  });
});

describe("what a session waits on", () => {
  const items: NonNullable<SessionRecord["waiting_for"]> = [
    { id: "a1", text: "Run the tests", since: AT, source: "agent", act: { kind: "run", target: "cargo test" } },
    { id: "ask:c1", text: "Which clock?", since: AT, source: "ask_card", act: { kind: "answer", target: "c1" }, options: [{ label: "Fake" }] },
  ];

  it("carries Fleet's merged list onto the session, with its choices", () => {
    const session = sessionOfRecord(record("a", { waiting_for: items }), undefined, beside());
    expect(session.waitingFor).toEqual(items);
  });

  it("carries none for a session with an empty list or one that ended", () => {
    expect(sessionOfRecord(record("a"), undefined, beside()).waitingFor).toBeUndefined();
    expect(sessionOfRecord(record("a", { state: "ended", waiting_for: items }), undefined, beside()).waitingFor).toBeUndefined();
  });
});

describe("what a session is doing", () => {
  const wire = { now: "Editing gate.rs", todos: [{ text: "Read the gate", state: "done" as const }, { text: "Run it on every Job", state: "doing" as const }] };

  it("carries what Fleet detected onto the session", () => {
    expect(sessionOfRecord(record("a", { activity: wire }), undefined, beside()).activity).toEqual({
      now: "Editing gate.rs",
      todos: wire.todos,
    });
  });

  it("carries none for a record without it, or one with nothing in it", () => {
    expect(sessionOfRecord(record("a"), undefined, beside()).activity).toBeUndefined();
    expect(activityOf({})).toBeUndefined();
    expect(activityOf({ todos: [] })).toBeUndefined();
  });

  it("is replaced by the next record, never merged: a dropped list and a cleared line do not come back", () => {
    const before = sessionOfRecord(record("a", { activity: wire }), undefined, beside());
    const later = sessionOfRecord(record("a", { activity: { now: "Running cargo check" } }), undefined, beside());
    expect(before.activity?.todos).toHaveLength(2);
    expect(later.activity).toEqual({ now: "Running cargo check" });
    expect(sessionOfRecord(record("a"), undefined, beside()).activity).toBeUndefined();
  });

  it("carries the agent's last sentence, the account of his message and the page it is building", () => {
    const account = { for: "m1", done: ["Added the page"], now: "Adding the switches", group: "g2", at: "2026-10-22T14:00:00Z" };
    const one = sessionOfRecord(record("a", { activity: { said: "Adding the switches now.", account, live: { url: "http://localhost:5173/", title: "Notifications" } } }), undefined, beside());
    expect(one.activity).toEqual({ said: "Adding the switches now." });
    expect(one.account).toEqual(account);
    expect(one.live).toEqual({ url: "http://localhost:5173/", title: "Notifications" });
  });

  it("gives an account with nothing done an empty done list", () => {
    const one = sessionOfRecord(record("a", { activity: { account: { for: "m1", now: "Reading the request", at: "2026-10-22T14:00:00Z" } } }), undefined, beside());
    expect(one.account).toEqual({ for: "m1", done: [], now: "Reading the request", at: "2026-10-22T14:00:00Z" });
    expect(one.activity).toBeUndefined();
  });
});

describe("the plan's limit and the context window", () => {
  const breakdown = {
    categories: [
      { name: "Messages", tokens: 23_600, kind: "used" as const },
      { name: "Free space", tokens: 913_000, kind: "free" as const },
    ],
    memory_files: [{ path: "NOTES.md", tokens: 2_100 }],
    mcp_tools: [{ name: "list_jobs", server: "armada-fleet", tokens: 2_600 }],
    total_tokens: 53_840,
    max_tokens: 1_000_000,
    autocompact_threshold: 967_000,
  };

  it("reads held_until as the reset a held session resumes at, and nothing for one that ended", () => {
    expect(sessionOfRecord(record("a", { held_until: "2026-10-12T22:40:00Z" }), undefined, beside()).limitedUntil).toBe("2026-10-12T22:40:00Z");
    expect(sessionOfRecord(record("a"), undefined, beside()).limitedUntil).toBeUndefined();
    expect(sessionOfRecord(record("a", { state: "ended", held_until: "2026-10-12T22:40:00Z" }), undefined, beside()).limitedUntil).toBeUndefined();
  });

  it("reads context_breakdown in the camelCase the view-model draws, and nothing without one", () => {
    expect(sessionOfRecord(record("a", { context_breakdown: breakdown }), undefined, beside()).contextBreakdown).toEqual({
      categories: breakdown.categories,
      memoryFiles: breakdown.memory_files,
      mcpTools: breakdown.mcp_tools,
      totalTokens: 53_840,
      maxTokens: 1_000_000,
      autocompactThreshold: 967_000,
    });
    expect(sessionOfRecord(record("a"), undefined, beside()).contextBreakdown).toBeUndefined();
  });
});

describe("what a session has built", () => {
  const build = { phase: "implement" as const, pieces: [{ id: "t1", text: "Dock layout", piece: "solid" as const }], stops: [{ id: "walk" as const }], forecast: ["walk" as const], stalled: false, workers: [] };

  it("carries the build Fleet derived, and none for a session that ended or has none", () => {
    expect(sessionOfRecord(record("b", { build }), undefined, beside()).build).toEqual(build);
    expect(sessionOfRecord(record("b", { state: "ended", build }), undefined, beside()).build).toBeUndefined();
    expect(sessionOfRecord(record("b"), undefined, beside()).build).toBeUndefined();
  });
});

describe("a session its agent was started by another to hold", () => {
  it("links the two sessions both ways, even after one ended", () => {
    const made = record("n", { attachments: [held("started_by", "old", {}, "spent")] });
    expect(sessionOfRecord(made, undefined, beside()).attachments).toEqual([{ kind: "started_by", id: "old" }]);
    const maker = record("old", { attachments: [held("started_to", "n", {}, "spent")] });
    expect(sessionOfRecord(maker, undefined, beside()).attachments).toEqual([{ kind: "started_to", id: "n" }]);
    expect(sessionOfRecord({ ...maker, state: "ended" }, undefined, beside()).attachments).toEqual([{ kind: "started_to", id: "n" }]);
  });

  it("reads the plan group a started Session was handed off both link rows", () => {
    const made = record("n", { attachments: [held("started_by", "old", { group: "g2" }, "spent")] });
    expect(sessionOfRecord(made, undefined, beside()).attachments).toEqual([{ kind: "started_by", id: "old", group: "g2" }]);
    const maker = record("old", { attachments: [held("started_to", "n", { group: "g2" }, "spent"), held("started_to", "m", {}, "spent")] });
    expect(sessionOfRecord(maker, undefined, beside()).attachments).toEqual([{ kind: "started_to", id: "n", group: "g2" }, { kind: "started_to", id: "m" }]);
  });

  it("keeps what a Session it follows did as a row in its thread", () => {
    const row: SessionRow = { kind: "linked", id: "l1", at: AT, session: { id: "n", title: "Port the clock" }, event: "needs" };
    expect(rowsOfThread("old", [row], () => undefined)).toMatchObject([{ id: "l1", kind: "linked", session: { id: "n", title: "Port the clock" }, event: "needs" }]);
  });

  it("keeps the Session the agent started as a row in its thread", () => {
    const row: SessionRow = { kind: "started", id: "st1", at: AT, session: { id: "n", title: "Port the clock" }, fork: true };
    expect(rowsOfThread("old", [row], () => undefined)).toMatchObject([{ id: "st1", kind: "started", session: { id: "n", title: "Port the clock" }, fork: true }]);
  });
});

describe("a question that lapsed", () => {
  const questions = [{ question: "Which night?", header: "Night", multi_select: false, options: [{ label: "Friday", description: "" }] }];
  const ask = (id: string, state: "unanswered" | "refused" | "kept" | "waiting" | "allowed_once", asked = questions): SessionRow => ({
    kind: "ask",
    id,
    at: AT,
    state,
    ask: { call: id, manifest_id: "armada", asked_at: AT, tool: "AskUserQuestion", detail: "", truncated: false, rule: "r", offers: ["refuse"], holding_for_seconds: 60, questions: asked },
  });
  const said = (id: string): SessionRow => ({ kind: "message", id, at: AT, from: { kind: "you" }, text: "ok" });

  it("is the last unanswered ask that carried questions", () => {
    expect(lapsedOf([said("1"), ask("2", "unanswered")])).toEqual({ id: "2", questions });
  });

  it("is a skipped question too, so a press by mistake can be undone", () => {
    expect(lapsedOf([said("1"), ask("2", "refused")])).toEqual({ id: "2", questions });
  });

  it("is gone once the person has said anything after it", () => {
    expect(lapsedOf([ask("1", "unanswered"), said("2")])).toBeUndefined();
  });

  it("stays when the question was kept for a walk, whatever he sends during it", () => {
    const note: SessionRow = { kind: "message", id: "n", at: AT, from: { kind: "you" }, text: "The header is off\nOn header, top" };
    const approved: SessionRow = { kind: "message", id: "a", at: AT, from: { kind: "you" }, text: "Approved: http://localhost:5173/" };
    expect(lapsedOf([ask("1", "kept"), note, approved])).toEqual({ id: "1", questions });
  });

  it("goes once his answer to a kept question is sent, or a newer ask comes", () => {
    const answer: SessionRow = { kind: "message", id: "r", at: AT, from: { kind: "you" }, text: lapsedAnswerText([{ question: "Which night?", chosen: ["Friday"] }]) };
    expect(lapsedOf([ask("1", "kept"), said("2"), answer])).toBeUndefined();
    expect(lapsedOf([ask("1", "kept"), ask("2", "waiting")])).toBeUndefined();
  });

  it("is superseded by a newer ask that carried questions", () => {
    expect(lapsedOf([ask("1", "unanswered"), ask("2", "allowed_once")])).toBeUndefined();
    expect(lapsedOf([ask("1", "unanswered"), ask("2", "waiting")])).toBeUndefined();
  });

  it("is not a permission that lapsed", () => {
    expect(lapsedOf([ask("1", "unanswered", [])])).toBeUndefined();
  });

  it("goes back to the agent a line a question", () => {
    expect(lapsedAnswerText([{ question: "Which night?", chosen: ["Friday", "Sat"] }, { question: "Where?", chosen: ["Home"] }])).toBe(
      'Answering your earlier question "Which night?": Friday, Sat\nAnswering your earlier question "Where?": Home',
    );
  });
});

const SCENE = {
  nodes: [
    { id: "a", kind: "box", x: 0, y: 0, title: "Bridge" },
    { id: "b", kind: "box", x: 300, y: 0, title: "Fleet" },
  ],
  edges: [{ id: "a-b", from: "a", to: "b", label: "start", step: 1 }],
};
const EARLIER = { nodes: [{ id: "a", kind: "box", x: 0, y: 0, title: "Bridge" }], edges: [] };

/** A sketch row as Fleet writes it: `detail` holds strings only, so `scene` is JSON text and `against` is JSON text of `{ title, scene }`. */
const sketchRow = (title: string, detail: Record<string, string>) => held("artifact", `sketch:${title}`, { form: "sketch", by: "agent", title, ...detail });
const againstText = (title: string, scene: string) => JSON.stringify({ title, scene });

describe("a sketch on the ledger", () => {
  it("is the agent's, read from the scene's JSON text, under its title", () => {
    const attachments = attachmentsOfRecord(record("a", { attachments: [sketchRow("Request flow", { scene: JSON.stringify(SCENE) })] }), beside());
    expect(attachments).toEqual([{ kind: "sketch", id: "sketch:Request flow", title: "Request flow", by: "agent", drawing: SCENE }]);
  });

  it("keeps the step an arrow carries", () => {
    const [one] = attachmentsOfRecord(record("a", { attachments: [sketchRow("Request flow", { scene: JSON.stringify(SCENE) })] }), beside());
    expect(one).toMatchObject({ drawing: { edges: [{ id: "a-b", step: 1 }] } });
  });

  it("holds the sketch it was drawn against, whole, with its title", () => {
    const [one] = attachmentsOfRecord(
      record("a", { attachments: [sketchRow("After", { scene: JSON.stringify(SCENE), against: againstText("Before", JSON.stringify(EARLIER)) })] }),
      beside(),
    );
    expect(one).toMatchObject({ against: { title: "Before", drawing: EARLIER } });
  });

  it("draws an empty frame for a scene that does not parse, and keeps the row", () => {
    const text = (scene: string) => attachmentsOfRecord(record("a", { attachments: [sketchRow("Flow", { scene })] }), beside());
    const empty = [{ kind: "sketch", id: "sketch:Flow", title: "Flow", by: "agent", drawing: { nodes: [], edges: [] } }];
    expect(text("{not json")).toEqual(empty);
    expect(text(JSON.stringify({ nodes: [{ id: "a", kind: "blob", x: 0, y: 0 }], edges: [] }))).toEqual(empty);
    // An arrow that joins a node the scene does not hold fails the whole of it, and nothing partial is drawn.
    expect(text(JSON.stringify({ nodes: SCENE.nodes, edges: [{ id: "x", from: "a", to: "gone" }] }))).toEqual(empty);
    expect(text(JSON.stringify({ ...SCENE, edges: [{ id: "a-b", from: "a", to: "b", step: "one" }] }))).toEqual(empty);
  });

  it("reads a sketch put to him, the drawing he is shown, and how he answered it", () => {
    const at = "2026-10-28T09:00:00.000Z";
    const sketch = (detail: Record<string, string>) => attachmentsOfRecord(record("a", { attachments: [sketchRow("Flow", { scene: JSON.stringify(SCENE), asked: "true", drawn: "r7", ...detail })] }), beside())[0];
    const open = sketch({});
    expect(open).toMatchObject({ asked: true, drawn: "r7" });
    expect(open).not.toHaveProperty("settled");
    expect(sketch({ settled: "approved", settled_at: at })).toMatchObject({ settled: { how: "approved", at } });
    expect(sketch({ settled: "approved", settled_at: at })).not.toHaveProperty("settled.words");
    expect(sketch({ settled: "sent_back", settled_at: at, words: "Smaller." })).toMatchObject({ settled: { how: "sent_back", words: "Smaller.", at } });
    const odds: Record<string, string>[] = [{ settled: "fixed", settled_at: at }, { settled: "approved" }];
    for (const odd of odds) expect(sketch(odd)).not.toHaveProperty("settled");
  });

  it("leaves a bad `against` off and keeps the sketch", () => {
    const [one] = attachmentsOfRecord(record("a", { attachments: [sketchRow("After", { scene: JSON.stringify(SCENE), against: againstText("Before", "{no") })] }), beside());
    expect(one).toMatchObject({ kind: "sketch", drawing: SCENE });
    expect(one).not.toHaveProperty("against");
    for (const against of ["{no", "[]", JSON.stringify({ scene: JSON.stringify(EARLIER) })]) {
      const [kept] = attachmentsOfRecord(record("a", { attachments: [sketchRow("After", { scene: JSON.stringify(SCENE), against })] }), beside());
      expect(kept).toMatchObject({ kind: "sketch", drawing: SCENE });
      expect(kept).not.toHaveProperty("against");
    }
  });

  it("stays on the ledger once the session has ended, as the other things it made do", () => {
    const given = { ...sketchRow("Flow", { scene: JSON.stringify(SCENE) }), state: "given_back" as const };
    expect(attachmentsOfRecord(record("a", { attachments: [given] }), beside())).toHaveLength(1);
  });

  it("is told from a page or a file by its form, and a form this build does not know is left out", () => {
    const unknown = held("artifact", "sketch:Flow", { form: "hologram" });
    expect(attachmentsOfRecord(record("a", { attachments: [unknown] }), beside())).toEqual([]);
  });

  it("is read from JSON text or from a scene already read, and from nothing else", () => {
    expect(sceneOfWire(JSON.stringify(SCENE))).toEqual(SCENE);
    expect(sceneOfWire(SCENE)).toEqual(SCENE);
    expect(sceneOfWire(undefined)).toBeUndefined();
    expect(sceneOfWire(7)).toBeUndefined();
  });
});

describe("a show_sketch call in the thread", () => {
  const call = (args: unknown, tool = "mcp__armada-fleet__show_sketch") => `${tool} ${JSON.stringify(args)}`;

  it("is read from the tool's name and its arguments as JSON", () => {
    expect(sketchCallOf(call({ title: "Request flow", scene: SCENE }))).toEqual({ title: "Request flow", scene: SCENE });
    expect(sketchCallOf(call({ title: " Request flow ", scene: JSON.stringify(SCENE) }, "show_sketch"))).toEqual({ title: "Request flow", scene: SCENE });
  });

  it("names the sketch it was drawn against, by title", () => {
    expect(sketchCallOf(call({ title: "After", scene: SCENE, against: "Before" }))).toMatchObject({ against: "Before" });
    expect(sketchCallOf(call({ title: "After", scene: SCENE, against: "" }))).not.toHaveProperty("against");
  });

  it("is not a call of another tool, and not one with nothing to draw", () => {
    expect(sketchCallOf(call({ title: "Flow", scene: SCENE }, "mcp__armada-fleet__show_window"))).toBeUndefined();
    expect(sketchCallOf("mcp__armada-fleet__show_sketch")).toBeUndefined();
    expect(sketchCallOf("mcp__armada-fleet__show_sketch {not json")).toBeUndefined();
    expect(sketchCallOf(call({ scene: SCENE }))).toBeUndefined();
    expect(sketchCallOf(call({ title: "Flow", scene: { nodes: "no", edges: [] } }))).toBeUndefined();
  });
});

describe("a message about one session", () => {
  const say = (id: string, text: string): SessionRow => ({ id, at: AT, kind: "message", from: { kind: "agent" }, text }) as SessionRow;
  const stable: Beside = { picture: () => undefined, sketches: [], pending: [], repository: (id) => id ?? "" };

  it("reads that session again and hands back every other one, and every unchanged row, as it was", () => {
    const memo = new SessionMemo();
    const records = [record("a"), record("b")];
    const threads: Record<string, SessionRow[]> = { a: [say("a1", "one"), say("a2", "two")], b: [say("b1", "one")] };
    const first = sessionsOfRecords(records, threads, () => stable, undefined, memo);

    // The next state arrives as new objects, one row more on `a`.
    const arrived = structuredClone({ records, threads: { ...threads, a: [...threads.a!, say("a3", "three")] } });
    const second = sessionsOfRecords(share(records, arrived.records), share(threads, arrived.threads), () => stable, undefined, memo);

    expect(second[1]).toBe(first[1]);
    expect(second[0]).not.toBe(first[0]);
    expect(second[0]!.rows).toHaveLength(3);
    expect(second[0]!.rows[0]).toBe(first[0]!.rows[0]);
    expect(second[0]!.rows[1]).toBe(first[0]!.rows[1]);
  });

  it("reads a session again when what stands beside it changed", () => {
    const memo = new SessionMemo();
    const records = [record("a")];
    const first = sessionsOfRecords(records, {}, () => stable, undefined, memo);
    const tag = { kind: "session", id: "s1", title: "A session" } as const;
    const again = sessionsOfRecords(records, {}, () => ({ ...stable, pending: [tag] }), undefined, memo);
    expect(first[0]!.pendingTags).toEqual([]);
    expect(again[0]!.pendingTags).toEqual([tag]);
  });
});

describe("a thread that is not read yet", () => {
  const say = (id: string, text: string): SessionRow => ({ id, at: AT, kind: "message", from: { kind: "agent" }, text }) as SessionRow;
  const stable: Beside = { picture: () => undefined, sketches: [], pending: [], repository: (id) => id ?? "" };

  it("is unread when no thread was held, and the same Session with a thread of no rows is not", () => {
    expect(sessionOfRecord(record("a"), undefined, beside()).transcript).toBe("unread");
    expect(sessionOfRecord(record("a"), [], beside()).transcript).toBeUndefined();
  });

  it("is refreshing when rows are held and the read of them has not answered", () => {
    const threads = { a: [say("a1", "one")] };
    const [made] = sessionsOfRecords([record("a")], threads, () => stable, undefined, undefined, ["a"]);
    expect(made!.transcript).toBe("refreshing");
    expect(made!.rows).toHaveLength(1);
  });

  it("says nothing once the rows are held and no read is out", () => {
    const threads = { a: [say("a1", "one")] };
    const [held] = sessionsOfRecords([record("a")], threads, () => stable);
    const [other] = sessionsOfRecords([record("a")], threads, () => stable, undefined, undefined, ["b"]);
    expect(held!.transcript).toBeUndefined();
    expect(other!.transcript).toBeUndefined();
  });

  it("stays unread while its read is out, since there is nothing held to refresh", () => {
    const [made] = sessionsOfRecords([record("a")], {}, () => stable, undefined, undefined, ["a"]);
    expect(made!.transcript).toBe("unread");
  });

  it("is read again when the read starts and when it answers, and not when only another Session is read", () => {
    const memo = new SessionMemo();
    const records = [record("a"), record("b")];
    const threads = { a: [say("a1", "one")], b: [say("b1", "one")] };
    const idle = sessionsOfRecords(records, threads, () => stable, undefined, memo, []);
    const reading = sessionsOfRecords(records, threads, () => stable, undefined, memo, ["a"]);
    expect(idle[0]!.transcript).toBeUndefined();
    expect(reading[0]!.transcript).toBe("refreshing");
    expect(reading[1]).toBe(idle[1]);

    const answered = sessionsOfRecords(records, threads, () => stable, undefined, memo, []);
    expect(answered[0]!.transcript).toBeUndefined();
    expect(answered[1]).toBe(idle[1]);
  });
});

describe("a Job handed on to a new one", () => {
  it("reads replaces and replaced_by as the two ends of the link", () => {
    const made = sessionOfRecord(record("new", { attachments: [held("replaces", "old", {}, "spent")] }), undefined, beside());
    const old = sessionOfRecord(record("old", { state: "ended", attachments: [held("replaced_by", "new", {}, "spent")] }), undefined, beside());
    expect(made.replaces).toBe("old");
    expect(made.replacedBy).toBeUndefined();
    expect(old.replacedBy).toBe("new");
    expect(old.dead).toBe("ended");
    expect(old.replaces).toBeUndefined();
  });

  it("draws the ring while handoff_starting stands and nothing once it is given back", () => {
    expect(sessionOfRecord(record("a", { attachments: [held("handoff_starting", "")] }), undefined, beside()).handoff).toBe("starting");
    expect(sessionOfRecord(record("a", { attachments: [held("handoff_starting", "", {}, "given_back")] }), undefined, beside()).handoff).toBeUndefined();
  });

  it("draws the failed band while handoff_failed stands, and clears it at the owner's next message", () => {
    const failed = held("handoff_failed", "2026-10-18T19:00:00Z", { reason: "No slot" });
    expect(sessionOfRecord(record("a", { attachments: [failed] }), undefined, beside()).handoff).toBe("failed");
    expect(sessionOfRecord(record("a", { attachments: [{ ...failed, state: "given_back" }] }), undefined, beside()).handoff).toBeUndefined();
  });

  it("reads a held start's facts onto the Job, and nothing once it has begun or ended", () => {
    const hosted = { turn: { state: "idle" as const }, mode: "auto" as const, running: false };
    const held = record("a", { hosted: { ...hosted, start: { asks_delivery: true, picked: { choice: "open_pr", from: "usual" } } } });
    expect(sessionOfRecord(held, undefined, beside()).start).toEqual({ asksDelivery: true, picked: { choice: "open_pr", from: "usual" } });
    expect(sessionOfRecord(record("a", { hosted: { ...hosted, start: { asks_delivery: false } } }), undefined, beside()).start).toEqual({ asksDelivery: false });
    const worded = record("a", { hosted: { ...hosted, start: { asks_delivery: true, picked: { choice: "auto_land", from: "default" }, message: "Port the clock" } } });
    expect(sessionOfRecord(worded, undefined, beside()).start).toEqual({ asksDelivery: true, picked: { choice: "auto_land", from: "default" }, message: "Port the clock" });
    expect(sessionOfRecord(record("a", { hosted }), undefined, beside()).start).toBeUndefined();
    expect(sessionOfRecord({ ...held, state: "ended" }, undefined, beside()).start).toBeUndefined();
  });

  it("carries Fleet's reason from the newest failed row, and none once the failure is cleared", () => {
    const older = held("handoff_failed", "2026-10-18T19:00:00Z", { reason: "old job did not end: the close was refused" });
    const newer = held("handoff_failed", "2026-10-18T19:05:00Z", { reason: "new job did not start: the manifest is gone" });
    expect(sessionOfRecord(record("a", { attachments: [older, newer] }), undefined, beside()).handoffReason).toBe("new job did not start: the manifest is gone");
    expect(sessionOfRecord(record("a", { attachments: [{ ...older, state: "given_back" }] }), undefined, beside()).handoffReason).toBeUndefined();
  });

  it("lets a failure outrank a starting row that was not given back, and puts none of these kinds on the ledger", () => {
    const both = record("a", { attachments: [held("handoff_starting", ""), held("handoff_failed", "2026-10-18T19:00:00Z")] });
    expect(sessionOfRecord(both, undefined, beside()).handoff).toBe("failed");
    expect(attachmentsOfRecord(both, beside())).toEqual([]);
  });
});

describe("a link a Job kept", () => {
  it("reads a link row as the address and who wrote it, from the person or the agent", () => {
    const attachments = attachmentsOfRecord(
      record("a", { attachments: [held("link", "https://example.com/rfc", { by: "you" }, "spent"), held("link", "https://example.com/docs", { by: "agent" }, "spent")] }),
      beside(),
    );
    expect(attachments).toEqual([
      { kind: "link", url: "https://example.com/rfc", by: "you" },
      { kind: "link", url: "https://example.com/docs", by: "agent" },
    ]);
  });

  it("keeps a link once the Job has ended", () => {
    const ended = record("a", { state: "ended", attachments: [held("link", "https://example.com/rfc", { by: "you" }, "spent")] });
    expect(sessionOfRecord(ended, undefined, beside()).attachments).toEqual([{ kind: "link", url: "https://example.com/rfc", by: "you" }]);
  });
});

describe("a Job's attachment history", () => {
  const history = (versions: AttachmentHistory["versions"]): AttachmentHistory => ({ versions });

  it("reads each version as the attachment it was at its instant, and what it followed as `from`", () => {
    const read = versionsOfHistory(
      history([
        { kind: "link", target: "https://example.com/rfc", state: "spent", detail: { by: "you" }, at: "2026-10-22T10:00:00Z", from_message: "m1" },
        { kind: "issue", target: "1887", state: "standing", detail: { state: "open" }, at: "2026-10-22T10:01:00Z", from_task: "t2" },
        { kind: "issue", target: "1887", state: "standing", detail: { state: "closed" }, at: "2026-10-22T10:30:00Z", from_message: "m2", from_task: "t4" },
      ]),
    );
    expect(read).toEqual([
      { at: "2026-10-22T10:00:00Z", attachment: { kind: "link", url: "https://example.com/rfc", by: "you" }, from: { message: "m1" } },
      { at: "2026-10-22T10:01:00Z", attachment: { kind: "issue", number: "1887", title: "", address: "", state: "open" }, from: { task: "t2" } },
      { at: "2026-10-22T10:30:00Z", attachment: { kind: "issue", number: "1887", title: "", address: "", state: "closed" }, from: { message: "m2", task: "t4" } },
    ]);
  });

  it("gives a version that followed nothing known no `from`, and drops a version this build has no screen for", () => {
    const read = versionsOfHistory(
      history([
        { kind: "link", target: "https://example.com/rfc", state: "spent", detail: { by: "agent" }, at: "2026-10-22T10:00:00Z" },
        { kind: "a_kind_from_later", target: "x", state: "standing", at: "2026-10-22T10:02:00Z" },
      ]),
    );
    expect(read).toEqual([{ at: "2026-10-22T10:00:00Z", attachment: { kind: "link", url: "https://example.com/rfc", by: "agent" } }]);
  });
});
