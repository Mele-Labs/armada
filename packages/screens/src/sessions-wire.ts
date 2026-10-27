// A session as Fleet serves it, read into the shape the Sessions screens draw.
//
// **One direction and no state**: the record, its thread and what the window holds beside them go
// in, a `Session` comes out. The ledger is Fleet's generic attachment (`kind`, `target`, `state`,
// `detail`), and what a screen draws is a slot beside its branch and a pull request with its Checks,
// so the pairing and the reading happen here, once, where a test can hold them. Nothing here reads a transcript: a row is drawn as it arrived and nothing is inferred from it.

import { isNotice } from "@armada/protocol";
import type { Attachment, AttachmentHistory, ContextBreakdown as WireContextBreakdown, HelmCallInFlight, ProposedField, SentFile, SessionActivity as WireActivity, SessionRecord, SessionRow as WireRow, SessionTag as WireTag, TimelineEvent, WaitingItem, WatchSummary, WorkflowSummary } from "@armada/protocol";

import type { ContextBreakdown } from "@armada/components/src/context-breakdown";
import { parseScene } from "@armada/components/src/compositions/SketchScene/scene";
import type { Scene } from "@armada/components/src/compositions/SketchScene/scene";
import { retroRowOf } from "./retro-row";
import { waitOfWire } from "./session-waits";
import { changeOfEvent, linesOfWire, settledRowsOf, workOf } from "./session-work";
import type { ReviewRefused } from "./session-work";
import { share } from "./share";
import type {
  Session,
  SessionActivity,
  SessionAsk,
  SessionAttachment,
  SessionChecks,
  SessionPullRequestState,
  SessionRow,
  SessionTag,
  SessionTrail,
} from "./draft/sessions";

/** What the window holds beside a record, which Fleet does not carry. */
export type Beside = {
  /** A picture a message carried, as an address the window can draw, where it has been read. */
  picture: (sessionId: string, fileId: string) => string | undefined;
  /** Sketches this window sent, which the wire holds only as the pictures they were rendered to. */
  sketches: readonly Extract<SessionAttachment, { kind: "sketch" }>[];
  /** Tags chosen and not yet sent. */
  pending: readonly SessionTag[];
  /** A repository as the picker names it, for the place a `sent` row says words went. Absent, the id stands. */
  repository?: (manifestId: string | undefined) => string;
  /** The workflows Fleet holds, which name the steps of the one a Session follows. */
  workflows?: readonly WorkflowSummary[];
  /** What happened across the Fleet, of which a value the person changed in a panel is a row of the thread. */
  events?: readonly TimelineEvent[];
  /** The Job's watches, which `watches.changed` carries and the record does not. Absent is none read. */
  watches?: readonly WatchSummary[];
  /** Picks of the proposer the person has changed here, which the record has not yet said. */
  unpicked?: ReadonlySet<ProposedField>;
  /** `features.workflows` is off, so the workflow picker, the steps and the proposer's mark for a workflow are not drawn. */
  workflowsOff?: boolean;
  /** Fleet's refusal of the last press of the plan's adversarial review, for the version it was pressed on. */
  reviewRefused?: ReviewRefused;
  /** Moves when something `picture` answers has changed without its identity doing so. */
  epoch?: number;
};

const NO_WORKFLOWS: readonly WorkflowSummary[] = [];
const NO_PICKS: ReadonlySet<ProposedField> = new Set();

/** What other sessions call a session: `s-` and the first eight characters of its id. */
export const addressOf = (id: string): string => `s-${id.slice(0, 8)}`;

function checksOf(detail: Record<string, string>): SessionChecks {
  const failing = detail["failing"] ?? "";
  // Fleet words a failure outside the gate as `pending` or `passed` with the names still in `failing`.
  const red = failing === "" ? {} : { red: failing };
  switch (detail["checks"]) {
    case "passed":
      return { state: "passed", ...red };
    case "failed":
      return { state: "failed", failing };
    default:
      return { state: "pending", ...red };
  }
}

function standingOf(detail: Record<string, string>, spent: boolean): SessionPullRequestState {
  if (spent || detail["state"] === "merged") return "merged";
  return detail["state"] === "draft" ? "draft" : "open";
}

/** What a scene that does not validate is drawn as: nothing. Never a partial drawing, never a repaired one. */
const NO_SCENE: Scene = { nodes: [], edges: [] };

/** A scene as Fleet keeps it, JSON text, or already read. `undefined` is any that `parseScene` would not draw. */
export function sceneOfWire(value: unknown): Scene | undefined {
  if (typeof value !== "string") return parseScene(value);
  try {
    return parseScene(JSON.parse(value));
  } catch {
    return undefined;
  }
}

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);

/** `detail.against`: JSON text of `{ title, scene }`. `undefined` when it is absent or is not that. */
function earlierOf(text: string | undefined): Record<string, unknown> | undefined {
  if (text === undefined) return undefined;
  try {
    const parsed: unknown = JSON.parse(text);
    return isRecord(parsed) ? parsed : undefined;
  } catch {
    return undefined;
  }
}

/**
 * A sketch the ledger holds, an artifact of form `sketch` keyed `sketch:<title>`. The scene is JSON text in
 * `detail`, and `against` the sketch it is read as a change to: a JSON string of `{ title, scene }`, the scene
 * itself JSON text again, so it is decoded in two steps (`detail` holds strings only). **A scene
 * that does not parse keeps its row and draws an empty frame**, so what the agent drew is still on the ledger.
 */
function sketchOf(one: Attachment): SessionAttachment {
  const detail: Record<string, string | undefined> = one.detail ?? {};
  const title = typeof detail["title"] === "string" && detail["title"] !== "" ? detail["title"] : one.target.replace(/^sketch:/, "");
  const earlier = earlierOf(detail["against"]);
  const earlierTitle = earlier !== undefined && typeof earlier["title"] === "string" ? earlier["title"] : undefined;
  const earlierScene = earlier === undefined ? undefined : sceneOfWire(earlier["scene"]);
  return {
    kind: "sketch",
    id: one.target,
    title,
    by: "agent",
    drawing: sceneOfWire(detail["scene"]) ?? NO_SCENE,
    ...(earlierTitle === undefined || earlierScene === undefined ? {} : { against: { title: earlierTitle, drawing: earlierScene } }),
  };
}

/** The name `show_sketch` is called by, however the harness prefixes a tool of a server. */
const SKETCH_CALL = /^(?:\S*__)?show_sketch\s+(\{[\s\S]*\})\s*$/;

/** The name `show_file` is called by, however the harness prefixes a tool of a server. */
const FILE_CALL = /^(?:\S*__)?show_file\s+(\S.*?)\s*$/;

/**
 * A `show_file` call as the thread's tool row carries it: the tool's name as the transcript gives it, then the path as the
 * agent wrote it, absolute or relative to where the job runs. `undefined` for any row that is not one.
 */
export function fileCallOf(text: string): { path: string } | undefined {
  const found = FILE_CALL.exec(text);
  return found === null ? undefined : { path: found[1]! };
}

/**
 * A `show_sketch` call as the thread's tool row carries it: the tool's name, then its arguments as JSON
 * (`title`, `scene`, and `against`, the title of the sketch it is read against). **`undefined` for any row
 * that is not one, and for one whose scene would not draw**, which stays the plain tool row it was.
 */
export function sketchCallOf(text: string): { title: string; scene: Scene; against?: string } | undefined {
  const found = SKETCH_CALL.exec(text);
  if (found === null) return undefined;
  let args: unknown;
  try {
    args = JSON.parse(found[1]!);
  } catch {
    return undefined;
  }
  if (!isRecord(args) || typeof args["title"] !== "string" || args["title"].trim() === "") return undefined;
  const scene = sceneOfWire(args["scene"]);
  if (scene === undefined) return undefined;
  const against = args["against"];
  return { title: args["title"].trim(), scene, ...(typeof against === "string" && against.trim() !== "" ? { against: against.trim() } : {}) };
}

/**
 * The revisions of a Job's document, from the `revisions` detail of its `document` row: a JSON list of paths, oldest first.
 * **Anything that is not a list of non-empty strings reads as none**, so a row Fleet wrote badly is a document with no pages and not a throw.
 */
export function revisionsOfDetail(kept: string | undefined): string[] {
  if (kept === undefined) return [];
  try {
    const read: unknown = JSON.parse(kept);
    return Array.isArray(read) && read.every((one) => typeof one === "string" && one !== "") ? (read as string[]) : [];
  } catch {
    return [];
  }
}

/** A pull request the ledger holds, with what Fleet last read of it. A merged one is `spent`; a closed one is let go. */
function pullRequestOf(one: Attachment): SessionAttachment | undefined {
  if (one.state === "given_back") return undefined;
  const detail = one.detail ?? {};
  const closes = (detail["closes"] ?? "").split(",").filter((one) => one !== "").map(Number).filter(Number.isInteger);
  const base = (detail["base"] ?? "").trim();
  return {
    kind: "pull_request",
    number: Number(one.target),
    title: detail["title"] ?? "",
    branch: detail["branch"] ?? "",
    address: detail["address"] ?? "",
    ...(detail["body"] === undefined || detail["body"] === "" ? {} : { body: detail["body"] }),
    checks: checksOf(detail),
    state: standingOf(detail, one.state === "spent"),
    auto: detail["auto_merge"] === "true",
    ...(detail["queued"] === "true" ? { queued: true as const } : {}),
    ...(closes.length === 0 ? {} : { closes }),
    ...(base === "" ? {} : { base }),
  };
}

/**
 * What a session holds, as the ledger draws it. **A slot and a branch are two rows on the wire and one
 * pair here**: the branch takes the slot the session holds in the same repository, so opening either
 * opens the slot's panel. A row Fleet or this build has no screen for is left out rather than guessed at.
 */
export function attachmentsOfRecord(record: Pick<SessionRecord, "attachments">, beside: Pick<Beside, "sketches">): SessionAttachment[] {
  // What a Session made stays on its ledger once it has ended, as a subagent's turn does.
  const held = record.attachments.filter((one) => one.state !== "given_back" || one.kind === "subagent" || one.kind === "artifact" || one.kind === "background");
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
      // Fleet's pull reading fills title, state and address after the row exists, so a fresh row is an open issue with only its number or key.
      case "issue": {
        const pulls = (detail["pull_requests"] ?? "").split(",").flatMap((part) => (/^[0-9]+$/.test(part.trim()) ? [Number(part.trim())] : []));
        out.push({
          kind: "issue",
          number: one.target,
          title: detail["title"] ?? "",
          address: detail["address"] ?? "",
          state: detail["state"] === "closed" ? "closed" : "open",
          ...(pulls.length === 0 ? {} : { pull_requests: pulls }),
          ...(detail["claimed"] === "true" ? { claimed: true as const } : {}),
        });
        break;
      }
      case "studio":
        out.push({ kind: "studio", id: one.target, title: detail["title"] ?? "Studio" });
        break;
      case "forked_to":
      case "forked_from":
        out.push(
          one.kind === "forked_to"
            ? { kind: "forked_to", id: one.target, ...(detail["moved"] === undefined ? {} : { moved: detail["moved"] }) }
            : {
                kind: "forked_from",
                id: one.target,
                ...(detail["at"] === undefined ? {} : { at: detail["at"] }),
                ...(detail["moved"] === undefined ? {} : { moved: detail["moved"] }),
                ...(detail["branch"] === undefined ? {} : { branch: detail["branch"] }),
              },
        );
        break;
      case "started_to":
      case "started_by":
        out.push({ kind: one.kind, id: one.target, ...(detail["group"] === undefined || detail["group"] === "" ? {} : { group: detail["group"] }) });
        break;
      case "subagent":
        out.push({
          kind: "subagent",
          id: one.target,
          task: detail["task"] ?? detail["description"] ?? "Subagent",
          state: one.state === "standing" ? "running" : "done",
          ...(detail["report"] === undefined ? {} : { report: detail["report"] }),
          ...(detail["type"] === undefined || detail["type"] === "" ? {} : { agentType: detail["type"] }),
          ...(detail["call"] === undefined || detail["call"] === "" ? {} : { call: detail["call"] }),
        });
        break;
      case "background": {
        const form = detail["form"];
        if (form !== "command" && form !== "monitor") break;
        const code = Number(detail["exit"]);
        const ended = one.state === "standing" ? undefined : one.changed_at;
        // A task the engine killed, or one whose process went with the Session, has no exit of its own.
        const state = one.state === "standing" ? "running" : one.state === "given_back" || detail["status"] === "killed" ? "stopped" : detail["status"] === "failed" ? "failed" : "done";
        out.push({
          kind: "background",
          id: one.target,
          form,
          task: detail["task"] ?? detail["command"] ?? "Background task",
          command: detail["command"] ?? "",
          started: one.since,
          state,
          ...(ended === undefined ? {} : { ended }),
          ...(state === "done" || state === "failed" ? (detail["exit"] === undefined || Number.isNaN(code) ? {} : { exit: code }) : {}),
          ...(detail["output"] === undefined ? {} : { output: detail["output"] }),
        });
        break;
      }
      case "artifact": {
        const form = detail["form"];
        if (form === "sketch") {
          out.push(sketchOf(one));
          break;
        }
        if (form !== "page" && form !== "file" && form !== "image" && form !== "doc" && form !== "document" && form !== "window") break;
        const revisions = form === "document" ? revisionsOfDetail(detail["revisions"]) : [];
        out.push({
          kind: "artifact",
          form,
          id: one.target,
          title: detail["title"] ?? one.target.slice(one.target.lastIndexOf("/") + 1),
          ...(form === "file" && detail["by"] === "agent" && detail["shown"] !== undefined ? { shown: detail["shown"], by: "agent" as const, ...(detail["written"] === "true" ? { written: true as const } : {}) } : {}),
          ...(revisions.length === 0 ? {} : { revisions }),
        });
        break;
      }
      case "link":
        out.push({ kind: "link", url: one.target, by: detail["by"] === "you" ? "you" : "agent" });
        break;
      default:
        break;
    }
  }
  return [...out, ...beside.sketches];
}

/**
 * A Job's attachment history as the draft keeps it: each version drawn as the ledger draws that attachment, with what it followed as `from`.
 * A version of a kind this build has no screen for is left out, as the ledger leaves out its row.
 */
export function versionsOfHistory(history: AttachmentHistory): NonNullable<Session["versions"]> {
  return history.versions.flatMap((one) => {
    const attachment = attachmentsOfRecord({ attachments: [{ ...one, since: one.at, changed_at: one.at }] }, { sketches: [] })[0];
    if (attachment === undefined) return [];
    const from = { ...(one.from_message === undefined ? {} : { message: one.from_message }), ...(one.from_task === undefined ? {} : { task: one.from_task }) };
    return [{ at: one.at, attachment, ...(Object.keys(from).length === 0 ? {} : { from }) }];
  });
}

/**
 * Where the work has been, off the same rows the ledger reads but keeping the ones it drops: a slot no longer
 * standing is a stay that ended (`changed_at`), and a pull request let go was still opened at its `since`. Nothing here
 * changes what `attachmentsOfRecord` draws.
 */
export function trailOfRecord(record: SessionRecord): SessionTrail {
  const bays = record.attachments
    .filter((one) => one.kind === "slot")
    .map((one) => ({ slot: Number(one.target), from: one.since, ...(one.state === "standing" ? {} : { until: one.changed_at }) }))
    .sort((a, b) => Date.parse(a.from) - Date.parse(b.from));
  const opened = Object.fromEntries(record.attachments.filter((one) => one.kind === "pr" && !Number.isNaN(Number(one.target))).map((one) => [Number(one.target), one.since]));
  return { started: record.started_at, ...(bays.length === 0 ? {} : { bays }), ...(Object.keys(opened).length === 0 ? {} : { opened }) };
}

const tagOf = (tag: WireTag): SessionTag => ({
  kind: tag.kind,
  id: tag.id,
  title: tag.title,
  ...(tag.url === undefined ? {} : { url: tag.url }),
});

function askOf(ask: HelmCallInFlight): SessionAsk {
  return {
    // The card is what a person allows, so it reads the command whole where `detail` was cut.
    command: ask.whole ?? (ask.detail === "" ? ask.tool : ask.detail),
    call: ask.call,
    offers: ask.offers,
    ...(ask.questions === undefined || ask.questions.length === 0 ? {} : { questions: ask.questions }),
    ...(ask.reaches === undefined ? {} : { reaches: ask.reaches }),
    ...(ask.reaches_to === undefined ? {} : { reachesTo: ask.reaches_to }),
  };
}

const LAPSE_ANSWER = "Answering your earlier question";

/** What the person says to the agent in place of the answer it did not wait for, a line a question. */
export const lapsedAnswerText = (answers: readonly { question: string; chosen: readonly string[] }[]): string =>
  answers.map((one) => `${LAPSE_ANSWER} "${one.question}": ${one.chosen.join(", ")}`).join("\n");

/**
 * The last ask with questions in it that nobody answered in time, that was skipped, or that was kept
 * because he was walking a page, while nothing the person said and no newer question came after it.
 * Reading back from the end, the first of those stops the search. **A kept one outlasts what he says
 * during the walk**, his notes and his Approve: only a message that answers it (`lapsedAnswerText`) clears it.
 */
export function lapsedOf(rows: readonly WireRow[]): Session["lapsed"] {
  let said = false;
  for (let at = rows.length - 1; at >= 0; at -= 1) {
    const row = rows[at]!;
    if (row.kind === "message" && row.from.kind === "you") {
      if (row.text.startsWith(LAPSE_ANSWER)) return undefined;
      said = true;
      continue;
    }
    if (row.kind !== "ask" || row.ask.questions === undefined || row.ask.questions.length === 0) continue;
    const lapsed = row.state === "kept" || (!said && (row.state === "unanswered" || row.state === "refused"));
    return lapsed ? { id: row.id, questions: row.ask.questions } : undefined;
  }
  return undefined;
}

const SETTLED: Record<string, string> = {
  ran_unasked: "ran without asking",
  allowed_once: "allowed once",
  allowed_and_remembered: "allowed and remembered",
  always_allowed: "always allowed",
  always_blocked: "always blocked",
  allowed_for_session: "allowed for this job",
  refused: "refused",
  unanswered: "no answer",
  kept: "kept for you",
  session_gone: "job ended",
};

/** The files a message carried, each with its picture where it is one and has been read. */
function filesOf(sessionId: string, files: readonly SentFile[], picture: Beside["picture"]) {
  return files.map((file) => {
    const src = file.media_type.startsWith("image/") ? picture(sessionId, file.id) : undefined;
    return { id: file.id, name: file.name, ...(src === undefined ? {} : { src }) };
  });
}

/** The thread, row by row. **A waiting ask is the card under the thread, so it is no row**; one answered is a line of what was decided. */
export function rowsOfThread(sessionId: string, rows: readonly WireRow[], picture: Beside["picture"], repository: NonNullable<Beside["repository"]> = (id) => id ?? ""): SessionRow[] {
  const out: SessionRow[] = [];
  for (const row of rows) {
    const at = row.at;
    switch (row.kind) {
      case "message":
        out.push({
          id: row.id,
          at,
          kind: "message",
          from: row.from,
          text: row.text,
          ...(row.files === undefined || row.files.length === 0 ? {} : { files: filesOf(sessionId, row.files, picture) }),
          ...(row.tags === undefined || row.tags.length === 0 ? {} : { tags: row.tags.map(tagOf) }),
          ...(isNotice(row.notice) ? { notice: row.notice } : {}),
        });
        break;
      case "sent":
        // Words Helm's dock sent on to a Session or a Job: the person's own, with the place they went.
        out.push({
          id: row.id,
          at,
          kind: "sent",
          text: row.text,
          target: { ...row.target, repository: repository(row.target.manifest_id) },
          ...(row.files === undefined || row.files.length === 0 ? {} : { files: filesOf(sessionId, row.files, picture) }),
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
      case "disputed":
        out.push({ id: row.id, at, kind: "disputed", notice: row.notice, why: row.why, spotted: row.spotted, ...(row.review === undefined ? {} : { review: row.review }) });
        break;
      case "lease":
        out.push({ id: row.id, at, kind: "lease", slot: row.slot, branch: row.branch });
        break;
      case "release":
        out.push({ id: row.id, at, kind: "release", slot: row.slot, branch: row.branch });
        break;
      case "check":
        out.push({ id: row.id, at, kind: "check", name: row.name, run: row.run, state: row.state, ...(row.target === undefined ? {} : { target: row.target }), ...(row.blame === undefined ? {} : { blame: row.blame }) });
        break;
      case "ran":
        out.push({ id: row.id, at, kind: "ran", command: row.command, code: row.code ?? null, tail: row.tail });
        break;
      case "requested":
        out.push({
          id: row.id,
          at,
          kind: "requested",
          by: row.by,
          method: row.method,
          url: row.url,
          ...(row.environment === undefined ? {} : { environment: row.environment }),
          headers: row.headers ?? [],
          ...(row.body === undefined ? {} : { body: row.body }),
          status: row.status ?? null,
          ms: row.ms,
          ...(row.error === undefined ? {} : { error: row.error }),
          response: row.response,
          truncated: row.truncated ?? false,
        });
        break;
      case "saved":
        out.push({ id: row.id, at, kind: "saved", name: row.name, collection: row.collection, collectionId: row.collection_id, request: row.request });
        break;
      case "window":
        out.push({ id: row.id, at, kind: "window", title: row.title, url: row.url, ...(row.ended_at === undefined ? {} : { ended: row.ended_at }) });
        break;
      case "started":
        out.push({ id: row.id, at, kind: "started", session: row.session, fork: row.fork });
        break;
      case "linked":
        out.push({ id: row.id, at, kind: "linked", session: row.session, event: row.event });
        break;
      case "waiter":
        out.push({
          id: row.id,
          at,
          kind: "waiter",
          session: row.session,
          event: row.event,
          ...(row.side === "waiting" ? { waiting: true as const } : {}),
          ...(row.says === undefined ? {} : { says: row.says }),
          ...(row.from === undefined ? {} : { from: row.from }),
          ...(row.to === undefined ? {} : { to: row.to }),
          ...(row.by === undefined ? {} : { by: row.by }),
        });
        break;
      case "retro":
        out.push(retroRowOf(row, at));
        break;
      // The open question is on the window's bar and under Waiting on you, so the thread keeps a line.
      case "window_ask":
        out.push({
          id: row.id,
          at,
          kind: "tool",
          text: `Asked in the window: ${row.question}${row.answer === undefined ? "" : ` — ${row.answer.replace(/\n/g, " · ")}`}`,
        });
        break;
      // The draft is answered on its card, so the row is kept whole.
      case "draft":
        out.push({
          id: row.id,
          at,
          kind: "draft",
          title: row.title,
          text: row.text,
          ...(row.before === undefined ? {} : { before: row.before }),
          ...(row.settled === undefined ? {} : { settled: row.settled }),
        });
        break;
      case "plan_changes":
        out.push({ id: row.id, at, kind: "suggest", lines: linesOfWire(row.lines) });
        break;
      case "ask":
        if (row.state !== "waiting") {
          out.push({
            id: row.id,
            at,
            kind: "tool",
            text: `${askOf(row.ask).command}: ${SETTLED[row.state] ?? row.state}`,
            ...(row.via === "phone" ? { fromPhone: true as const } : {}),
          });
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

/** A session, whole. `rows` are the thread where it was opened and none where it was not; `reading` while Fleet has not answered a read of it. */
export function sessionOfRecord(record: SessionRecord, rows: readonly WireRow[] | undefined, beside: Beside, reading = false): Session {
  const hosted = record.hosted;
  const terminal = record.terminal;
  const model = hosted?.model ?? terminal?.model;
  const effort = hosted?.effort ?? terminal?.effort;
  const mode = hosted?.mode ?? terminal?.mode;
  const asked = hosted?.asked ?? terminal?.asked;
  const ended = record.state === "ended";
  // A session that ended holds nothing, so it owns nothing: what it keeps is the link to its forks and starts, and what it made, to be read.
  const attachments = attachmentsOfRecord(record, beside).filter(
    (one) => !ended || one.kind === "forked_to" || one.kind === "forked_from" || one.kind === "started_to" || one.kind === "started_by" || one.kind === "pull_request" || one.kind === "artifact" || one.kind === "link",
  );
  // A Job handed on to a new one. The ledger kinds are `docs/practices/protocol/a-job-hands-off-to-a-new-one.md`; a failed hand off outranks a starting one that was not given back.
  const replaces = record.attachments.find((one) => one.kind === "replaces" && one.state === "spent")?.target;
  const replacedBy = record.attachments.find((one) => one.kind === "replaced_by" && one.state === "spent")?.target;
  const standing = (kind: string) => record.attachments.some((one) => one.kind === kind && one.state === "standing");
  // Fleet's bare reason for the failed row: the newest standing one, since each failure writes a row of its own.
  const failedRows = record.attachments.filter((one) => one.kind === "handoff_failed" && one.state === "standing");
  const handoffReason = ended ? undefined : failedRows[failedRows.length - 1]?.detail?.reason;
  const handoff = ended ? undefined : standing("handoff_failed") ? ("failed" as const) : standing("handoff_starting") ? ("starting" as const) : undefined;
  const dead = ended ? ("ended" as const) : hosted === undefined && terminal?.listening !== true ? ("quiet" as const) : undefined;
  const title = cleanTitle(record.title);
  const thread = rows === undefined ? [] : withChanges(rowsOfThread(record.id, rows, beside.picture, beside.repository), rows, record.id, beside.events, record.settled);
  const lapsed = rows === undefined || ended ? undefined : lapsedOf(rows);
  const activity = activityOf(record.activity);
  const account = accountOf(record.activity);
  const work = workOf(record, { workflows: beside.workflows ?? NO_WORKFLOWS, attachments, rows, drawn: thread, unpicked: beside.unpicked ?? NO_PICKS, workflowsOff: beside.workflowsOff === true, ...(beside.reviewRefused === undefined ? {} : { reviewRefused: beside.reviewRefused }) });
  return {
    id: record.id,
    address: addressOf(record.id),
    ...(title === undefined ? {} : { title }),
    ...(record.manifest_id === undefined ? {} : { manifestId: record.manifest_id }),
    ...(record.cwd === "" ? {} : { cwd: record.cwd }),
    ...(hosted === undefined ? { terminal: true as const } : {}),
    ...(record.helm === true ? { helm: true as const } : {}),
    ...(record.mod === true ? { mod: true as const } : {}),
    ...(dead === undefined ? {} : { dead }),
    ...(handoff === undefined ? {} : { handoff }),
    ...(handoff !== "failed" || handoffReason === undefined ? {} : { handoffReason }),
    ...(replacedBy === undefined ? {} : { replacedBy }),
    ...(replaces === undefined ? {} : { replaces }),
    ...(hosted?.paused === undefined || ended ? {} : { paused: true as const }),
    ...(record.auto_land === true && !ended ? { autoLand: true as const } : {}),
    // A thread nobody opened has no rows to say the session was ever spoken in: the title and a finished turn say it.
    blank: attachments.length === 0 && title === undefined && record.last_turn_at === undefined && thread.length === 0,
    attachments,
    trail: trailOfRecord(record),
    rows: thread,
    // `[]` is also what an unread thread draws, so the difference from one with no messages is said here.
    ...(rows === undefined ? { transcript: "unread" as const } : reading ? { transcript: "refreshing" as const } : {}),
    turn:
      hosted?.turn.state === "working"
        ? { state: "working", ...(hosted.turn.woken_by === undefined ? {} : { wokenBy: hosted.turn.woken_by }) }
        : { state: "idle" },
    ...(record.last_turn_at === undefined ? {} : { lastTurnAt: record.last_turn_at }),
    ...(asked === undefined ? {} : { asked: askOf(asked) }),
    ...(lapsed === undefined ? {} : { lapsed }),
    pendingTags: beside.pending,
    ...(model === undefined ? {} : { model }),
    ...(hosted?.resolved_model === undefined ? {} : { resolvedModel: hosted.resolved_model }),
    ...(effort === undefined ? {} : { effort }),
    ...(mode === undefined ? {} : { mode }),
    ...(terminal?.commands === undefined ? {} : { commands: terminal.commands }),
    ...(record.mod_out_of_date === true ? { modOutOfDate: true as const } : {}),
    ...(record.held_until === undefined || ended ? {} : { limitedUntil: record.held_until }),
    ...(record.context_breakdown === undefined ? {} : { contextBreakdown: contextBreakdownOf(record.context_breakdown) }),
    ...(record.loadout_summary === undefined ? {} : { loadout: record.loadout_summary }),
    ...(record.usage.context_tokens === undefined || record.usage.context_window === undefined
      ? {}
      : { context: { tokens: record.usage.context_tokens, window: record.usage.context_window } }),
    ...(activity === undefined ? {} : { activity }),
    ...(account === undefined ? {} : { account }),
    ...(record.activity?.live === undefined ? {} : { live: record.activity.live }),
    ...(ended || record.build === undefined ? {} : { build: record.build }),
    ...(ended || (record.waiting_for?.length ?? 0) === 0 ? {} : { waitingFor: record.waiting_for }),
    ...(ended || record.done_since === undefined ? {} : { done: { since: record.done_since } }),
    ...(ended || record.standing === undefined ? {} : { standing: record.standing }),
    ...(ended || hosted?.start === undefined ? {} : { start: { asksDelivery: hosted.start.asks_delivery, ...(hosted.start.picked === undefined ? {} : { picked: hosted.start.picked }), ...(hosted.start.message === undefined ? {} : { message: hosted.start.message }) } }),
    ...(ended || record.waits === undefined ? {} : { waits: waitOfWire(record.waits) }),
    ...(ended || (record.later?.length ?? 0) === 0 ? {} : { later: (record.later ?? []).map((one) => ({ id: one.id, text: one.sentence, at: one.said_at, ...(one.confirmed === true ? { confirmed: true } : {}) })) }),
    ...(ended || record.held_open_by === undefined ? {} : { heldOpenBy: record.held_open_by }),
    ...(beside.watches === undefined || beside.watches.length === 0 ? {} : { watches: beside.watches }),
    ...(work === undefined ? {} : { work }),
  };
}

/** The values the person changed through a panel, which the agent was told of, and the concerns he settled, as rows of the thread where they happened. */
function withChanges(thread: SessionRow[], rows: readonly WireRow[], sessionId: string, events: readonly TimelineEvent[] | undefined, settled: readonly WaitingItem[] | undefined): SessionRow[] {
  const changes = [...settledRowsOf(settled, thread)];
  for (const event of events ?? []) {
    if (event.subject.kind !== "session" || event.subject.id !== sessionId) continue;
    const change = changeOfEvent(event.kind, event.says);
    if (change !== undefined) changes.push({ at: event.at, row: { id: `event:${event.id}`, at: event.at, kind: "change" as const, ...change } });
  }
  if (changes.length === 0) return thread;
  changes.sort((a, b) => a.at.localeCompare(b.at));
  const when = new Map(rows.map((one) => [one.id, one.at]));
  const out = [...thread];
  for (const change of changes) {
    const at = out.findIndex((one) => (when.get(one.id) ?? "") > change.at);
    out.splice(at < 0 ? out.length : at, 0, change.row);
  }
  return out;
}

/**
 * What Fleet detected a Session doing, as the view-model draws it. **Whole every time**: a row's activity
 * replaces the last one, so a todo the agent dropped does not outlive its list. Nothing to say is `undefined`.
 */
export function activityOf(wire: WireActivity | undefined): SessionActivity | undefined {
  if (wire === undefined) return undefined;
  const todos = wire.todos ?? [];
  if (wire.now === undefined && todos.length === 0 && wire.said === undefined) return undefined;
  return {
    ...(wire.now === undefined ? {} : { now: wire.now }),
    ...(todos.length === 0 ? {} : { todos }),
    ...(wire.said === undefined ? {} : { said: wire.said }),
  };
}

/** The account of his last message Fleet's small model keeps, with an empty done list where nothing is done yet. */
function accountOf(wire: WireActivity | undefined): Session["account"] {
  const account = wire?.account;
  if (account === undefined) return undefined;
  return {
    for: account.for,
    done: account.done ?? [],
    ...(account.now === undefined ? {} : { now: account.now }),
    ...(account.next === undefined ? {} : { next: account.next }),
    ...(account.group === undefined ? {} : { group: account.group }),
    at: account.at,
  };
}

/** What fills a window, in the camelCase the view-model draws. */
function contextBreakdownOf(wire: WireContextBreakdown): ContextBreakdown {
  return {
    categories: wire.categories,
    memoryFiles: wire.memory_files,
    mcpTools: wire.mcp_tools,
    totalTokens: wire.total_tokens,
    maxTokens: wire.max_tokens,
    ...(wire.autocompact_threshold === undefined ? {} : { autocompactThreshold: wire.autocompact_threshold }),
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
  memo?: SessionMemo,
  reading: readonly string[] = [],
): Session[] {
  const since = now - ENDED_LISTED_DAYS * 24 * 60 * 60 * 1000;
  return records.map((one) => {
    const old = one.state === "ended" && Date.parse(one.last_seen_at) < since;
    const read = reading.includes(one.id);
    const build = (): Session => {
      const session = sessionOfRecord(one, threads[one.id], beside(one.id), read);
      return old ? { ...session, older: true as const } : session;
    };
    return memo === undefined ? build() : memo.of(one, threads[one.id], beside(one.id), old, build, read);
  });
}

/**
 * What each Session was last drawn from, so a message about one Session reads only that one. **A hit needs the very
 * same record, rows and `Beside` parts, so the caller hands in values that do not change identity unless they changed**
 * (`share.ts`), including a `picture` and `repository` that stay one function. A miss is built again and shares what it
 * can with the last build, so a row that did not change is the same object it was.
 */
export class SessionMemo {
  private readonly held = new Map<string, { record: SessionRecord; rows: readonly WireRow[] | undefined; beside: Beside; old: boolean; reading: boolean; session: Session }>();

  of(record: SessionRecord, rows: readonly WireRow[] | undefined, beside: Beside, old: boolean, build: () => Session, reading = false): Session {
    const last = this.held.get(record.id);
    if (last !== undefined && last.record === record && last.rows === rows && last.old === old && last.reading === reading && sameBeside(last.beside, beside)) return last.session;
    const session = share(last?.session, build());
    this.held.set(record.id, { record, rows, beside, old, reading, session });
    return session;
  }

  /** Lets go of every Session not in `ids`: one that left the list must not be held. */
  keep(ids: ReadonlySet<string>): void {
    for (const id of this.held.keys()) if (!ids.has(id)) this.held.delete(id);
  }
}

const sameBeside = (a: Beside, b: Beside): boolean =>
  a.picture === b.picture && a.sketches === b.sketches && a.pending === b.pending && a.repository === b.repository && a.epoch === b.epoch && a.workflows === b.workflows && a.events === b.events && a.watches === b.watches && a.unpicked === b.unpicked && a.workflowsOff === b.workflowsOff && a.reviewRefused === b.reviewRefused;
