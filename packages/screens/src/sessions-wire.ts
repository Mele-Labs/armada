// A session as Fleet serves it, read into the shape the Sessions screens draw.
//
// **One direction and no state**: the record, its thread and what the window holds beside them go
// in, a `Session` comes out. The ledger is Fleet's generic attachment (`kind`, `target`, `state`,
// `detail`), and what a screen draws is a slot beside its branch, a pull request with its Checks, a
// Job read off the Board — so the pairing and the reading happen here, once, where a test can hold
// them. Nothing here reads a transcript: a row is drawn as it arrived and nothing is inferred from it.

import type { Attachment, HelmCallInFlight, JobSummary, SessionRecord, SessionRow as WireRow, SessionTag as WireTag } from "@armada/protocol";

import { clock } from "./duration";
import type {
  Session,
  SessionAsk,
  SessionAttachment,
  SessionChecks,
  SessionJobState,
  SessionPullRequestState,
  SessionRow,
  SessionTag,
} from "./draft/sessions";

/** What the window holds beside a record, which Fleet does not carry. */
export type Beside = {
  /** The Board, which a Job row of the ledger reads its title, number, state and branch from. */
  jobs: readonly JobSummary[];
  /** A picture a message carried, as an address the window can draw, where it has been read. */
  picture: (sessionId: string, fileId: string) => string | undefined;
  /** Sketches this window sent, which the wire holds only as the pictures they were rendered to. */
  sketches: readonly Extract<SessionAttachment, { kind: "sketch" }>[];
  /** Tags chosen and not yet sent. */
  pending: readonly SessionTag[];
};

/** What other sessions call a session: `s-` and the first eight characters of its id. */
export const addressOf = (id: string): string => `s-${id.slice(0, 8)}`;

/** A Job's number, which Fleet carries only inside its handle: `12-the-drone-count-is-wrong`. */
export function numberOf(job: Pick<JobSummary, "handle">): number {
  const found = /^(\d+)-/.exec(job.handle);
  return found === null ? 0 : Number(found[1]);
}

/** Where a Job stands, in the six words a Session's ledger has. */
export function jobStateOf(status: string): SessionJobState {
  switch (status) {
    case "awaiting_review":
    case "awaiting_repair":
    case "awaiting_attestation":
      return "review";
    case "completed_success":
      return "landed";
    case "escalated":
    case "completed_failed":
      return "escalated";
    case "piloted":
      return "piloted";
    case "killed":
    case "rejected":
    case "superseded":
      return "superseded";
    default:
      return "running";
  }
}

function checksOf(detail: Record<string, string>): SessionChecks {
  const failing = detail["failing"] ?? "";
  switch (detail["checks"]) {
    case "passed":
      return { state: "passed" };
    case "failed":
      return { state: "failed", failing };
    default:
      return { state: "pending" };
  }
}

function standingOf(detail: Record<string, string>, spent: boolean): SessionPullRequestState {
  if (spent || detail["state"] === "merged") return "merged";
  return detail["state"] === "draft" ? "draft" : "open";
}

/**
 * The Job a slot or branch came with, where the session was handed it by a take over. Fleet writes
 * `detail.handed` as `job <id>`; a Job the Board no longer holds names nothing.
 */
function handedOf(detail: Record<string, string>, jobs: readonly JobSummary[]): { handed?: { job: number } } {
  const id = /^job (.+)$/.exec(detail["handed"] ?? "")?.[1];
  const job = id === undefined ? undefined : jobs.find((row) => row.id === id);
  return job === undefined ? {} : { handed: { job: numberOf(job) } };
}

/** A pull request the ledger holds, with what Fleet last read of it. A merged one is `spent`; a closed one is let go. */
function pullRequestOf(one: Attachment): SessionAttachment | undefined {
  if (one.state === "given_back") return undefined;
  const detail = one.detail ?? {};
  return {
    kind: "pull_request",
    number: Number(one.target),
    title: detail["title"] ?? "",
    branch: detail["branch"] ?? "",
    address: detail["address"] ?? "",
    checks: checksOf(detail),
    state: standingOf(detail, one.state === "spent"),
    auto: detail["auto_merge"] === "true",
  };
}

/**
 * What a session holds, as the ledger draws it. **A slot and a branch are two rows on the wire and one
 * pair here**: the branch takes the slot the session holds in the same repository, so opening either
 * opens the slot's panel. A row Fleet or this build has no screen for is left out rather than guessed at.
 */
export function attachmentsOfRecord(record: SessionRecord, beside: Pick<Beside, "jobs" | "sketches">): SessionAttachment[] {
  // What a Session made stays on its ledger once it has ended, as a subagent's turn does.
  const held = record.attachments.filter((one) => one.state !== "given_back" || one.kind === "subagent" || one.kind === "artifact");
  const slots = held.filter((one) => one.kind === "slot" && one.state === "standing");
  const out: SessionAttachment[] = [];
  for (const one of held) {
    const detail = one.detail ?? {};
    switch (one.kind) {
      case "slot":
        if (one.state === "standing") out.push({ kind: "slot", slot: Number(one.target), ...handedOf(detail, beside.jobs) });
        break;
      case "branch": {
        if (one.state !== "standing") break;
        const slot = slots.find((s) => s.manifest_id === one.manifest_id) ?? slots[0];
        out.push({ kind: "branch", name: one.target, slot: slot === undefined ? 0 : Number(slot.target), ...handedOf(detail, beside.jobs) });
        break;
      }
      case "pr": {
        const pr = pullRequestOf(one);
        if (pr !== undefined) out.push(pr);
        break;
      }
      case "job": {
        const job = beside.jobs.find((row) => row.id === one.target);
        // A Job Fleet has forgotten has no row to open, and nothing on the ledger names it any other way.
        if (job === undefined) break;
        out.push({
          kind: "job",
          id: job.id,
          number: numberOf(job),
          title: job.title,
          state: jobStateOf(job.status),
          branch: job.branch ?? "",
          ...(detail["looking"] === "true" ? { looking: true as const } : {}),
          // A person's word closed it. Said apart from a Job that passed its gates.
          ...(job.piloted?.exit === "attested" ? { attested: true as const } : {}),
          ...(job.status === "piloted" && job.piloted?.session_id !== record.id ? { pilotedElsewhere: true as const } : {}),
        });
        break;
      }
      case "studio":
        out.push({ kind: "studio", id: one.target, title: detail["title"] ?? "Studio" });
        break;
      case "forked_to":
      case "forked_from":
        out.push(one.kind === "forked_to" ? { kind: "forked_to", id: one.target } : { kind: "forked_from", id: one.target });
        break;
      case "subagent":
        out.push({
          kind: "subagent",
          id: one.target,
          task: detail["task"] ?? detail["description"] ?? "Subagent",
          state: one.state === "standing" ? "running" : "done",
          ...(detail["report"] === undefined ? {} : { report: detail["report"] }),
        });
        break;
      case "artifact": {
        const form = detail["form"];
        if (form !== "page" && form !== "file" && form !== "doc") break;
        out.push({ kind: "artifact", form, id: one.target, title: detail["title"] ?? one.target.slice(one.target.lastIndexOf("/") + 1) });
        break;
      }
      default:
        break;
    }
  }
  return [...out, ...beside.sketches];
}

const tagOf = (tag: WireTag): SessionTag => ({
  kind: tag.kind,
  id: tag.id,
  title: tag.title,
  ...(tag.job === undefined ? {} : { job: { ...tag.job, state: jobStateOf(tag.job.state) } }),
});

function askOf(ask: HelmCallInFlight): SessionAsk {
  return {
    command: ask.detail === "" ? ask.tool : ask.detail,
    call: ask.call,
    offers: ask.offers,
    ...(ask.questions === undefined || ask.questions.length === 0 ? {} : { questions: ask.questions }),
  };
}

const SETTLED: Record<string, string> = {
  ran_unasked: "ran without asking",
  allowed_once: "allowed once",
  allowed_and_remembered: "allowed and remembered",
  refused: "refused",
  unanswered: "no answer",
  session_gone: "session ended",
};

/** What the Drone said it tried, a line to an item with any list marker taken off. */
const linesOf = (text: string): string[] =>
  text
    .split("\n")
    .map((line) => line.replace(/^\s*(?:[-*]|\d+[.)])\s+/, "").trim())
    .filter((line) => line !== "");

/** The thread, row by row. **A waiting ask is the card under the thread, so it is no row**; one answered is a line of what was decided. */
export function rowsOfThread(sessionId: string, rows: readonly WireRow[], picture: Beside["picture"]): SessionRow[] {
  const out: SessionRow[] = [];
  for (const row of rows) {
    const at = clock(row.at);
    switch (row.kind) {
      case "message":
        out.push({
          id: row.id,
          at,
          kind: "message",
          from: row.from,
          text: row.text,
          ...(row.files === undefined || row.files.length === 0
            ? {}
            : {
                files: row.files.map((file) => {
                  const src = file.media_type.startsWith("image/") ? picture(sessionId, file.id) : undefined;
                  return { id: file.id, name: file.name, ...(src === undefined ? {} : { src }) };
                }),
              }),
          ...(row.tags === undefined || row.tags.length === 0 ? {} : { tags: row.tags.map(tagOf) }),
        });
        break;
      case "tool":
        out.push({ id: row.id, at, kind: "tool", text: row.text });
        break;
      case "command":
        out.push({ id: row.id, at, kind: "command", text: row.text });
        break;
      case "compaction":
        out.push({ id: row.id, at, kind: "compaction", text: row.text });
        break;
      case "lease":
        out.push({ id: row.id, at, kind: "lease", slot: row.slot, branch: row.branch });
        break;
      case "handoff":
        out.push({
          id: row.id,
          at,
          kind: "handoff",
          job: { number: row.number, title: row.title },
          slot: row.slot ?? 0,
          branch: row.branch,
          ...(row.step === undefined ? {} : { step: row.step }),
          attempts: row.attempts,
          refusals: row.refusals ?? [],
          plan: { outside: row.plan.outside ?? [], unwritten: row.plan.unwritten ?? [] },
          ...(row.narrative === undefined
            ? {}
            : { narrative: { trying_to: row.narrative.trying_to, blocked_by: row.narrative.blocked_by, tried: linesOf(row.narrative.tried) } }),
        });
        break;
      case "ask":
        if (row.state !== "waiting") {
          out.push({ id: row.id, at, kind: "tool", text: `${askOf(row.ask).command}: ${SETTLED[row.state] ?? row.state}` });
        }
        break;
    }
  }
  return out;
}

// The mod's `titleOf` rule (plugins/armada/hooks/facts.ts), applied on read so a title stored before
// the mod stripped markup shows clean. A title that cleans to nothing is no title.
const MACHINE_BLOCK = /<((?:local-)?command(?:-\w+)+|system-reminder)(?:\s[^>]*)?>[\s\S]*?(?:<\/\1>|$)/g;
const WRAPPER_TAG = /<\/?[a-z]+(?:-\w+)+(?:\s[^>]*)?>/gi;

/** A stored title without the harness's markup, flattened to one line; nothing where nothing is left. */
export function cleanTitle(title: string | undefined): string | undefined {
  if (title === undefined) return undefined;
  const flat = title.replace(MACHINE_BLOCK, "").replace(WRAPPER_TAG, "").replace(/\s+/g, " ").trim();
  return flat === "" ? undefined : flat;
}

/** A session, whole. `rows` are the thread where it was opened and none where it was not. */
export function sessionOfRecord(record: SessionRecord, rows: readonly WireRow[] | undefined, beside: Beside): Session {
  const hosted = record.hosted;
  const terminal = record.terminal;
  const model = hosted?.model ?? terminal?.model;
  const effort = hosted?.effort ?? terminal?.effort;
  const mode = hosted?.mode ?? terminal?.mode;
  const ended = record.state === "ended";
  // A session that ended holds nothing, so it owns nothing: what it keeps is the link to its forks.
  const attachments = attachmentsOfRecord(record, beside).filter((one) => !ended || one.kind === "forked_to" || one.kind === "forked_from");
  const dead = ended ? ("ended" as const) : hosted === undefined && terminal?.listening !== true ? ("quiet" as const) : undefined;
  const title = cleanTitle(record.title);
  const thread = rows === undefined ? [] : rowsOfThread(record.id, rows, beside.picture);
  return {
    id: record.id,
    address: addressOf(record.id),
    ...(title === undefined ? {} : { title }),
    ...(hosted === undefined ? { terminal: true as const } : {}),
    ...(dead === undefined ? {} : { dead }),
    // A thread nobody opened has no rows to say the session was ever spoken in: the title and a finished turn say it.
    blank: attachments.length === 0 && title === undefined && record.last_turn_at === undefined && thread.length === 0,
    attachments,
    rows: thread,
    turn:
      hosted?.turn.state === "working"
        ? { state: "working", ...(hosted.turn.woken_by === undefined ? {} : { wokenBy: hosted.turn.woken_by }) }
        : { state: "idle" },
    ...(record.last_turn_at === undefined ? {} : { lastTurn: clock(record.last_turn_at).replace(/:\d\d$/, ""), lastTurnAt: record.last_turn_at }),
    ...(hosted?.asked === undefined ? {} : { asked: askOf(hosted.asked) }),
    pendingTags: beside.pending,
    ...(model === undefined ? {} : { model }),
    ...(effort === undefined ? {} : { effort }),
    ...(mode === undefined ? {} : { mode }),
    ...(terminal?.commands === undefined ? {} : { commands: terminal.commands }),
    ...(record.mod_out_of_date === true ? { modOutOfDate: true as const } : {}),
  };
}

/** How long an ended session stays in the list after it was last seen. Older ones are found by search. */
export const ENDED_LISTED_DAYS = 7;

/**
 * The sessions the window draws: the ones still open, and the ones that ended, which offer Fork and own
 * nothing. An ended one last seen before `ENDED_LISTED_DAYS` ago is `older`: the list leaves it off and a
 * search still finds it.
 */
export function sessionsOfRecords(
  records: readonly SessionRecord[],
  threads: Readonly<Record<string, readonly WireRow[]>>,
  beside: (id: string) => Beside,
  now: number = Date.now(),
): Session[] {
  const since = now - ENDED_LISTED_DAYS * 24 * 60 * 60 * 1000;
  return records.map((one) => {
    const session = sessionOfRecord(one, threads[one.id], beside(one.id));
    return one.state === "ended" && Date.parse(one.last_seen_at) < since ? { ...session, older: true as const } : session;
  });
}
