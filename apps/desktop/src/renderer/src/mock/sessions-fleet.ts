// A Fleet that serves Sessions, for a window to talk to as it talks to the real one.
//
// **Not the mock's Sessions** (`sessions/script.ts`), which are fixtures behind the draft seam. This
// stands where main stands: it holds the sessions and the threads, answers the capabilities the
// preload carries, and publishes what main would publish. The window above it is the real one, so a
// test through `App` reaches the same code a person's press does. What Fleet does on its own clock —
// an agent's reply, a row, a pull request's Checks moving — is a method the test calls.

import type {
  AnswerSessionAsk,
  Attachment,
  AttachmentState,
  HelmCallInFlight,
  JobSummary,
  PullRequestState,
  SendSessionMessage,
  SessionRecord,
  SessionRow,
  RenameSession,
  SessionSubagent,
  TuneSession,
} from "@armada/protocol";

import type { SessionActed } from "../../../shared/api/sessions";
import type { Scenario } from "./moment";
import type { FleetHandle } from "./moment";

const AT = "2026-10-07T13:48:02.000Z";

/** An attachment as Fleet writes it. */
export function held(kind: string, target: string, detail: Record<string, string> = {}, state: AttachmentState = "standing"): Attachment {
  return { kind, manifest_id: "armada", target, state, detail, since: AT, changed_at: AT };
}

/** A session Fleet hosts, idle, with nothing leased. */
export function hosted(id: string, change: Partial<SessionRecord> = {}): SessionRecord {
  return {
    id,
    harness: "a_harness",
    origin: "bridge",
    manifest_id: "armada",
    cwd: "/Users/user/code/armada",
    state: "live",
    started_at: AT,
    last_seen_at: AT,
    usage: {},
    attachments: [],
    hosted: { turn: { state: "idle" }, mode: "auto", running: false },
    ...change,
  };
}

/** A session from a person's terminal: on the ledger, and with no thread. */
export function terminal(id: string, change: Partial<SessionRecord> = {}): SessionRecord {
  const { hosted: _hosted, ...rest } = hosted(id, change);
  // A terminal whose mod is asking, as one a person is at: it takes a message, and offers no Fork.
  return { terminal: { listening: true }, ...rest, origin: "terminal" };
}

/** A call the agent is held on, with the three offers. */
export function asking(command: string, call = "call-1"): HelmCallInFlight {
  return {
    call,
    manifest_id: "armada",
    asked_at: AT,
    tool: "Bash",
    detail: command,
    truncated: false,
    rule: `Bash(${command}:*)`,
    offers: ["allow_once", "allow_and_remember", "refuse"],
    holding_for_seconds: 600,
  };
}

/** The pull request as the forge shows it, in the words the ledger's `detail` carries. */
export const pullRequest = (number: number, detail: Partial<Record<string, string>> = {}): Attachment =>
  held("pr", String(number), {
    state: "open",
    auto_merge: "false",
    checks: "passed",
    failing: "",
    title: "Fix the flaky store test",
    branch: "fix/flaky-store",
    address: `https://forge.example/pull/${number}`,
    ...(detail as Record<string, string>),
  });

export type Calls = {
  started: number;
  piloted: { jobId: string; outcome: string }[];
  exited: { jobId: string; exit: string; note?: string }[];
  sent: SendSessionMessage[];
  answered: AnswerSessionAsk[];
  tuned: TuneSession[];
  renamed: RenameSession[];
  closed: string[];
  forked: string[];
  /** The Sessions the Retro press was pressed on. */
  retroed: string[];
  pressed: { sessionId: string; number: number; press: string }[];
  watched: string[];
  /** The addresses a page view was shown on, and how many times it was taken away. */
  pages: string[];
  pagesHidden: number;
};

export class FakeSessionsFleet {
  /** Who hears Esc pressed in the page's view. */
  private readonly escape = new Set<() => void>();
  /** Esc, pressed while the page's view has focus. */
  pageEscape(): void {
    this.escape.forEach((on) => on());
  }
  readonly calls: Calls = { started: 0, piloted: [], exited: [], sent: [], answered: [], tuned: [], renamed: [], closed: [], forked: [], retroed: [], pressed: [], watched: [], pages: [], pagesHidden: 0 };
  private records: SessionRecord[];
  private threads: Record<string, SessionRow[]>;
  private fleet: FleetHandle | undefined;
  /** The Board's rows, which a take over and an exit change as Fleet does. */
  private jobs: JobSummary[] = [];
  /** Set to have the next take over, or the next exit, refused the way Fleet refuses: a 409 and nothing changed. */
  refuses: { code: string; message: string } | undefined;
  /** Set to have the next message refused, the way Fleet refuses one: the error says why and nothing is sent. */
  refusesSend: { code: string; message: string } | undefined;
  /** How long a retro takes to write, so a walk can see it being written. */
  retroTakes = 1500;
  /** Hears a retro being written, to put it where the Retros page reads from. */
  onRetro: ((sessionId: string, title: string | undefined) => void) | undefined;
  /** Each subagent's thread as its transcript stands, by subagent id. A test changes it to let one run on. */
  subagents: Record<string, SessionSubagent> = {};
  private minted = 0;
  private rowed = 0;

  constructor(records: SessionRecord[] = [], threads: Record<string, SessionRow[]> = {}) {
    this.records = records;
    this.threads = threads;
  }

  /** `base`, with Sessions served: what main publishes once Fleet has answered `list_sessions`, and the capabilities over them. */
  scenario(base: Scenario): Scenario {
    this.jobs = base.state.jobs;
    return {
      ...base,
      state: { ...base.state, sessions: { state: "read", sessions: this.records }, sessionThreads: {} },
      behaves: (fleet) => {
        this.fleet = fleet;
        return {
          startSession: async () => {
            this.calls.started += 1;
            const record = hosted(`01SESSION${String((this.minted += 1)).padStart(8, "0")}ABCDEFGH`);
            this.set([record, ...this.records]);
            return { ok: true, value: record };
          },
          pilotJob: async (jobId, outcome) => {
            this.calls.piloted.push({ jobId, outcome });
            const refused = this.refusal();
            if (refused !== undefined) return refused;
            const job = this.jobs.find((row) => row.id === jobId)!;
            const id = `01PILOT${String((this.minted += 1)).padStart(8, "0")}ABCDEFGHIJ`;
            const number = Number(/^(\d+)-/.exec(job.handle)?.[1] ?? 0);
            const record = hosted(id, {
              title: job.title,
              attachments: [
                held("slot", "3", { handed: `job ${jobId}` }),
                held("branch", job.branch ?? "", { handed: `job ${jobId}` }),
                held("job", jobId),
              ],
            });
            this.threads = {
              ...this.threads,
              [id]: [
                {
                  kind: "handoff",
                  id: "handoff1",
                  at: AT,
                  job_id: jobId,
                  number,
                  title: job.title,
                  reason: outcome,
                  slot: 3,
                  branch: job.branch ?? "",
                  step: { id: "regression_verify", label: "Verify the fix" },
                  attempts: 3,
                  refusals: ["No test covers the retry cap, retry.rs:41"],
                  plan: { declared: true, outside: ["crates/retry/src/loop.rs"] },
                },
              ],
            };
            this.setJobs(this.jobs.map((row) => (row.id === jobId ? { ...row, status: "piloted", piloted: { reason: outcome, session_id: id, since: AT } } : row)));
            this.set([record, ...this.records]);
            this.publishThreads();
            return { ok: true, value: record };
          },
          exitPilot: async (jobId, exit, note) => {
            this.calls.exited.push({ jobId, exit, ...(note === undefined ? {} : { note }) });
            const refused = this.refusal();
            if (refused !== undefined) return refused.outcome;
            const how = { submit: ["queued", "submitted"], attest: ["completed_success", "attested"], supersede: ["superseded", "superseded"] }[exit] as [string, string];
            this.setJobs(
              this.jobs.map((row) =>
                row.id === jobId ? { ...row, status: how[0], piloted: { ...row.piloted!, exit: how[1], ended_at: AT, ...(note === undefined ? {} : { note }) } } : row,
              ),
            );
            // The Session gives the slot and branch back, as Fleet writes it.
            this.set(
              this.records.map((one) => ({
                ...one,
                attachments: one.attachments.map((a) => (a.detail?.["handed"] === `job ${jobId}` ? { ...a, state: "given_back" as const } : a)),
              })),
            );
            return { ok: true };
          },
          sendSessionMessage: async (send) => {
            if (this.refusesSend !== undefined) return { ok: false, outcome: { ok: false, why: "refused", error: this.refusesSend } as never };
            this.calls.sent.push(send);
            this.row(send.session_id, { kind: "message", id: this.rowId(), at: AT, from: { kind: "you" }, text: send.text });
            return this.change(send.session_id, (one) => ({ ...one, hosted: { ...one.hosted!, turn: { state: "working" } } }));
          },
          answerSessionAsk: async (answer) => {
            this.calls.answered.push(answer);
            return this.change(answer.session_id, (one) => {
              const { asked: _asked, ...rest } = one.hosted!;
              return { ...one, hosted: rest };
            });
          },
          tuneSession: async (tune) => {
            this.calls.tuned.push(tune);
            return this.change(tune.session_id, (one) => ({
              ...one,
              hosted: { ...one.hosted!, mode: tune.mode, ...(tune.model === undefined ? {} : { model: tune.model }), ...(tune.effort === undefined ? {} : { effort: tune.effort }) },
            }));
          },
          renameSession: async (rename) => {
            this.calls.renamed.push(rename);
            return this.change(rename.session_id, (one) => ({ ...one, title: rename.title }));
          },
          forkSession: async (sessionId) => {
            this.calls.forked.push(sessionId);
            const old = this.records.find((one) => one.id === sessionId);
            if (old === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
            // Fleet's own refusal: a session that is still running is not forked.
            if (old.state === "live" && (old.hosted !== undefined || old.terminal?.listening === true)) {
              return { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.session_fork_live", message: `session ${sessionId} is still running, and only an ended one is forked` } } as never };
            }
            const id = `01FORK${String((this.minted += 1)).padStart(8, "0")}ABCDEFGHIJ`;
            const record = hosted(id, { ...(old.title === undefined ? {} : { title: old.title }), attachments: [held("forked_from", sessionId, {}, "spent")] });
            this.set([record, ...this.records.map((one) => (one.id === sessionId ? { ...one, attachments: [...one.attachments, held("forked_to", id, {}, "spent")] } : one))]);
            return { ok: true, value: record };
          },
          // **Served only where the scenario hears retros**, so no other Sessions scenario grows a press.
          ...(this.onRetro === undefined
            ? {}
            : {
                retroSession: async (sessionId: string) => {
                  this.calls.retroed.push(sessionId);
                  await new Promise((done) => setTimeout(done, this.retroTakes));
                  this.onRetro?.(sessionId, this.records.find((one) => one.id === sessionId)?.title);
                  return { ok: true as const };
                },
              }),
          closeSession: async (sessionId) => {
            this.calls.closed.push(sessionId);
            return this.change(sessionId, (one) => ({ ...one, state: "ended" }));
          },
          watchSession: async (sessionId) => {
            this.calls.watched.push(sessionId);
            this.threads = { ...this.threads, [sessionId]: this.threads[sessionId] ?? [] };
            this.publishThreads();
          },
          readSessionSubagent: async (_sessionId, subagentId) => {
            const thread = this.subagents[subagentId];
            return thread === undefined ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, value: thread };
          },
          readSessionFile: async () => ({ ok: true, bytes: new Uint8Array([137, 80, 78, 71]), type: "image/png" }),
          openSessionFile: async () => ({ ok: true }),
          readSessionArtifact: async () => ({ ok: true, bytes: new TextEncoder().encode("# Store clock"), type: "text/markdown" }),
          showSessionPage: async (_sessionId, address) => {
            this.calls.pages.push(address);
            return { ok: true };
          },
          moveSessionPage: async () => undefined,
          onSessionPageEscape: (on) => {
            this.escape.add(on);
            return () => void this.escape.delete(on);
          },
          hideSessionPage: async () => {
            this.calls.pagesHidden += 1;
          },
          pressPullRequest: async (sessionId, number, press) => {
            this.calls.pressed.push({ sessionId, number, press });
            if (press === "review") return { ok: true, value: { job_id: "01REVIEWJOB", address: `https://forge.example/pull/${number}`, session_id: sessionId } };
            if (press === "merge") {
              const refused = this.merge(sessionId, number);
              if (refused !== undefined) return refused;
            }
            const now = this.records.find((one) => one.id === sessionId)?.attachments.find((one) => one.kind === "pr" && one.target === String(number));
            const state: PullRequestState = {
              manifest_id: "armada",
              number,
              state: press === "merge" ? "merged" : "open",
              auto_merge: press === "auto_merge",
              checks: { state: "passed" },
              title: now?.detail?.["title"] ?? "",
              branch: now?.detail?.["branch"] ?? "",
              address: now?.detail?.["address"] ?? "",
            };
            return { ok: true, value: state };
          },
        };
      },
    };
  }

  private refusal(): { ok: false; outcome: never } | undefined {
    if (this.refuses === undefined) return undefined;
    return { ok: false, outcome: { ok: false, why: "refused", error: this.refuses } as never };
  }

  private setJobs(jobs: JobSummary[]): void {
    this.jobs = jobs;
    this.fleet?.publish({ jobs });
  }

  private rowId(): string {
    return `row${(this.rowed += 1)}`;
  }

  private merge(sessionId: string, number: number): SessionActed<never> | undefined {
    const row = this.records.find((one) => one.id === sessionId)?.attachments.find((one) => one.kind === "pr" && one.target === String(number));
    if (row?.detail?.["checks"] !== "passed") {
      return { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.merge_checks_not_passed", message: "The checks have not passed." } } as never };
    }
    this.change(sessionId, (one) => ({
      ...one,
      attachments: one.attachments.map((a) => (a === row ? { ...a, state: "spent" as const, detail: { ...a.detail, state: "merged" } } : a)),
    }));
    return undefined;
  }

  private set(records: SessionRecord[]): void {
    this.records = records;
    this.fleet?.publish({ sessions: { state: "read", sessions: records } });
  }

  private publishThreads(): void {
    this.fleet?.publish({ sessionThreads: this.threads });
  }

  private change(sessionId: string, how: (one: SessionRecord) => SessionRecord): SessionActed {
    const found = this.records.find((one) => one.id === sessionId);
    if (found === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const record = how(found);
    this.set(this.records.map((one) => (one.id === sessionId ? record : one)));
    return { ok: true, value: record };
  }

  /** `session.changed`: a session, whole, as Fleet publishes it after any fact. */
  changed(record: SessionRecord): void {
    this.set(this.records.some((one) => one.id === record.id) ? this.records.map((one) => (one.id === record.id ? record : one)) : [record, ...this.records]);
  }

  /** `session.row`: one row, appended or replaced by its id, where the thread is held. */
  row(sessionId: string, row: SessionRow): void {
    const held = this.threads[sessionId];
    // Not opened: main holds no thread for it and drops the row.
    if (held === undefined) return;
    this.threads = { ...this.threads, [sessionId]: held.some((one) => one.id === row.id) ? held.map((one) => (one.id === row.id ? row : one)) : [...held, row] };
    this.publishThreads();
  }

  /** The agent says something in a thread. */
  says(sessionId: string, text: string): void {
    this.row(sessionId, { kind: "message", id: this.rowId(), at: AT, from: { kind: "agent" }, text });
  }
}
