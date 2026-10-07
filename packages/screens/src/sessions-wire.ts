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
  const held = record.attachments.filter((one) => one.state !== "given_back" || one.kind === "subagent");
  const slots = held.filter((one) => one.kind === "slot" && one.state === "standing");
  const out: SessionAttachment[] = [];
  for (const one of held) {
    const detail = one.detail ?? {};
    switch (one.kind) {
      case "slot":
        if (one.state === "standing") out.push({ kind: "slot", slot: Number(one.target) });
        break;
      case "branch": {
        if (one.state !== "standing") break;
        const slot = slots.find((s) => s.manifest_id === one.manifest_id) ?? slots[0];
        out.push({ kind: "branch", name: one.target, slot: slot === undefined ? 0 : Number(slot.target) });
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
        });
        break;
      }
      case "studio":
        out.push({ kind: "studio", id: one.target, title: detail["title"] ?? "Studio" });
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
  return { command: ask.detail === "" ? ask.tool : ask.detail, call: ask.call, offers: ask.offers };
}

const SETTLED: Record<string, string> = {
  ran_unasked: "ran without asking",
  allowed_once: "allowed once",
  allowed_and_remembered: "allowed and remembered",
  refused: "refused",
  unanswered: "no answer",
  session_gone: "session ended",
};

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
      case "lease":
        out.push({ id: row.id, at, kind: "lease", slot: row.slot, branch: row.branch });
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

/** A session, whole. `rows` are the thread where it was opened and none where it was not. */
export function sessionOfRecord(record: SessionRecord, rows: readonly WireRow[] | undefined, beside: Beside): Session {
  const hosted = record.hosted;
  const attachments = attachmentsOfRecord(record, beside);
  const thread = rows === undefined ? [] : rowsOfThread(record.id, rows, beside.picture);
  return {
    id: record.id,
    address: addressOf(record.id),
    ...(record.title === undefined ? {} : { title: record.title }),
    ...(hosted === undefined ? { terminal: true as const } : {}),
    // A thread nobody opened has no rows to say the session was ever spoken in: the title and a finished turn say it.
    blank: attachments.length === 0 && record.title === undefined && record.last_turn_at === undefined && thread.length === 0,
    attachments,
    rows: thread,
    turn:
      hosted?.turn.state === "working"
        ? { state: "working", ...(hosted.turn.woken_by === undefined ? {} : { wokenBy: hosted.turn.woken_by }) }
        : { state: "idle" },
    ...(record.last_turn_at === undefined ? {} : { lastTurn: clock(record.last_turn_at).replace(/:\d\d$/, "") }),
    ...(hosted?.asked === undefined ? {} : { asked: askOf(hosted.asked) }),
    pendingTags: beside.pending,
    ...(hosted?.model === undefined ? {} : { model: hosted.model }),
    ...(hosted?.effort === undefined ? {} : { effort: hosted.effort }),
    ...(hosted === undefined ? {} : { mode: hosted.mode }),
  };
}

/** The sessions the window draws: the ones still open, the person's own first. A closed session holds nothing, so it owns nothing. */
export function sessionsOfRecords(records: readonly SessionRecord[], threads: Readonly<Record<string, readonly WireRow[]>>, beside: (id: string) => Beside): Session[] {
  return records.filter((one) => one.state === "live").map((one) => sessionOfRecord(one, threads[one.id], beside(one.id)));
}
