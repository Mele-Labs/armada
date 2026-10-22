// The HTTP half of Bridge's one connection: one function, and the two shape
// checks its answers need.
//
// It sits beside `connection.ts` rather than inside it because the connection
// is a socket, a runtime file and a state machine, and a fetch with a timeout
// is none of those. Nothing here holds state, so nothing here can drift from
// what the socket believes.

import type {
  CheckOutputRead,
  FrameRead,
  Holdings,
  Outcome,
  TransportFault,
} from "@armada/protocol";
import type { FleetLimits, ManifestReading, MergeLines } from "@armada/protocol";
import type { Preferences } from "@armada/protocol";
import type { Lessons, LessonsRead, Retro, RetroRead, RetroSubject } from "@armada/protocol";
import type {
  AskLessonAnswer,
  AskLessonRead,
  AskTurn,
  LessonAsk,
  LessonReview,
  LessonReviewRead,
} from "@armada/screens/src/lessons/review-wire";
import type { ServerList } from "@armada/protocol";
import type { CheckOutput } from "@armada/protocol";
import type { LeftOutWorkflow, ManifestSummary, ModelChoices, RepositoryList, WorkflowSummary } from "@armada/protocol";
import { pendingAt, refusedWith, sentOf } from "@armada/protocol";
import { Socket } from "node:net";
import { Agent, fetch, type Dispatcher } from "undici";
import type { Picked } from "./picked";
import { HOST } from "./runtime-file";

/**
 * The type-of-service byte is advisory, and this makes Node treat it that way.
 *
 * Node's bundled `undici` sets it at the top of every HTTP/1 write, from
 * `writeH1`, with nothing around the call, and `Socket.setTypeOfService` throws
 * whatever errno came back on every platform but Windows. macOS answers
 * `EINVAL` once a socket has been shut down — `setsockopt(2)` says so — and a
 * peer resetting between the handshake and that write leaves a socket the
 * kernel has torn down and JavaScript still reads as open. The throw is held by
 * nobody: an uncaught exception in main, and a whole suite lost after every
 * test in it passed. #613.
 *
 * **Only this syscall**, so the unhandled-error class is untouched. If the
 * socket really is dead the write behind it fails and undici reports that.
 */
function typeOfServiceStaysAdvisory(): void {
  const socket = Socket.prototype as unknown as {
    setTypeOfService?: (this: Socket, tos: number) => unknown;
  };
  const setting = socket.setTypeOfService;
  if (setting === undefined) return;
  socket.setTypeOfService = function (this: Socket, tos: number): unknown {
    try {
      return setting.call(this, tos);
    } catch (cause) {
      if ((cause as NodeJS.ErrnoException | undefined)?.syscall !== "setTypeOfService") throw cause;
      return this;
    }
  };
}

typeOfServiceStaysAdvisory();

/**
 * What every request to Fleet goes through, so **Bridge's own wait is the only
 * bound on it**: `waitMs`, or nothing at `NO_WAIT`. With no dispatcher, undici
 * cuts any answer whose headers take over 300 000 ms as `fetch failed`, signal
 * or not — on 4 Oct 2026 a `rerun_checks` Fleet passed after five and a half
 * minutes showed as `bridge.command.unreachable`. Zero turns both bounds off.
 *
 * undici's own `fetch` rather than the global one: a dispatcher must be the
 * same undici as the fetch using it, and the global is the runtime's — 7.24 in
 * Electron 40, another major in the Node the tests run on.
 */
const FLEET: Dispatcher = new Agent({ headersTimeout: 0, bodyTimeout: 0 });

/** The margin over Fleet's own bound on a command, so Fleet gives up first and says so in words. */
const COMMAND_MARGIN_MS = 5_000;

/**
 * How long an ordinary command waits before it is a transport failure.
 *
 * **Derived, not guessed.** `#712`: Fleet's own bound on a plain command is
 * `timeouts.commandSeconds` in settings.json — fifteen seconds as shipped —
 * and this is that plus a five-second margin, so a timeout here means Fleet
 * gave up first and answered in words, the way `MODEL_CALL_MS` already reads
 * for the two routes with a model call inside them. **A live binding**: it
 * follows the setting from the first read of settings.json on
 * (`followCommandSeconds`), and every importer reads the value now in force.
 */
export let COMMAND_MS = 15_000 + COMMAND_MARGIN_MS;

/** Follow `timeouts.commandSeconds`. A figure that is not a positive number leaves the wait as it was. */
export function followCommandSeconds(seconds: number): void {
  if (!Number.isFinite(seconds) || seconds <= 0) return;
  COMMAND_MS = seconds * 1000 + COMMAND_MARGIN_MS;
}

/**
 * What a route waits when a model call is inside the request.
 *
 * **Five seconds is a bound on a socket, not on a question.** Every route but
 * two answers off the store and returns in milliseconds, and `COMMAND_MS` is
 * sized for those. `POST /jobs/:id/rerun_gate` asks the Judge inside the
 * request, and Fleet's own bound on one call is `PROVISIONAL_JUDGE_BUDGET` in
 * `crates/armada/src/serve.rs` — two minutes, and a gate re-run may make
 * several. Bridge was giving it five seconds.
 *
 * **Deliberately longer than Fleet's bound**, so that Fleet's own refusal —
 * with a code, a run id and a chain — is what arrives rather than Bridge's
 * abort. Nothing generates this from the Rust constant; the two are coupled by
 * this comment.
 */
export const MODEL_CALL_MS = 150_000;
/**
 * No wait at all. **What `POST /jobs/from_request` takes**, and Kit's install
 * and remove of a piece, which run commands under Fleet's own bound (`kit.ts`).
 *
 * A wait is a guess about how long is too long, made by the side that knows
 * least and enforced on somebody who was not asked. It was worth making while
 * Bridge had nothing else to offer — a socket held open forever with a blank
 * form in front of it is worse than an abort. It is not worth making now:
 * `proposal.moved` says how far the call has got and `stopProposal` kills it,
 * so the person watching decides, with Fleet's budget as the backstop.
 *
 * **Bridge giving up first is strictly worse than either.** It throws away
 * Fleet's coded answer, and it leaves the call running and spending with nobody
 * to read what it decides.
 */
export const NO_WAIT = 0;

/**
 * How long a frame waits. **Longer than a command, for the one reason a bound
 * is ever raised here**: what is on the wire is a file rather than a row, and
 * `COMMAND_MS` is sized for a read that answers off the store in milliseconds.
 *
 * Fleet opens one file it has already checked is there, so the work is the
 * transfer — and the number a reader compares this against is `KeptFrame.bytes`
 * on the row they pressed, which is why the bound is generous rather than
 * derived from it. A screenshot that took thirty seconds to come back over
 * loopback is a machine in trouble, not a large image.
 */
const FRAME_MS = 30_000;

/** Bridge names itself on every request, so Fleet tells a press here from an agent's (23.12). */
export const CALLER = { "x-armada-caller": "bridge" } as const;

/** A Manifest-scoped call on a repository with no `armada.yml` yet — `Picked.manifest` answered `null`. */
export const NOT_SET_UP = { ok: false, why: "not_set_up" } as const;

/** What came back: a body to read, or the refusal to render. */
export type Answer = { ok: true; body: unknown } | { ok: false; outcome: Outcome };

/**
 * One request to Fleet. Loopback plus the port from the runtime file.
 *
 * `waitMs` is the wait, and it is a parameter rather than one constant because
 * two routes put a model call inside the request — see [`MODEL_CALL_MS`]. Every
 * other caller takes the default and is right to.
 *
 * **Every failure here names the route it was about.** A transport failure
 * carries no `WireError`, so what a person quoting it has is whatever this
 * builds: which of the three it was, what was asked, and how long Bridge
 * waited. See `TransportFault` for why the three are three.
 */
export async function ask(
  port: number,
  method: "GET" | "POST",
  path: string,
  body?: unknown,
  waitMs: number = COMMAND_MS,
  // `FLEET` everywhere but a test, which hands in undici's bounds at its own scale.
  through: Dispatcher = FLEET,
): Promise<Answer> {
  // A pending route's body rides on the fault, so `Not implemented` says
  // what the act carried — `sentOf`.
  const asked = {
    method,
    path,
    ...(body !== undefined && pendingAt(method, path) !== null ? { sent: sentOf(body) } : {}),
  };
  try {
    const answer = await fetch(`http://${HOST}:${port}${path}`, {
      method,
      headers: body === undefined ? CALLER : { ...CALLER, "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      // Zero is no signal at all rather than an immediate abort — see
      // `NO_WAIT`. `AbortSignal.timeout(0)` would fire on the next tick.
      ...(waitMs === NO_WAIT ? {} : { signal: AbortSignal.timeout(waitMs) }),
      dispatcher: through,
    });
    const text = await answer.text();
    if (!answer.ok) {
      return { ok: false, outcome: refusedWith(answer.status, text, asked) };
    }
    return { ok: true, body: JSON.parse(text) };
  } catch (cause) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    // `AbortSignal.timeout` rejects with a `TimeoutError`, and it is the one
    // failure here that says the request went out. Told apart by name rather
    // than by the message, which is the vendor's wording and not a contract.
    const fault: TransportFault =
      cause instanceof Error && cause.name === "TimeoutError"
        ? { ...asked, why: "timed_out", waitedMs: waitMs }
        : { ...asked, why: "unreachable" };
    return { ok: false, outcome: { ok: false, why: "transport", detail, fault } };
  }
}

/**
 * The workflows, the Manifests and the models Fleet holds. Three calls, being
 * three operations in the inventory.
 *
 * A failed one keeps what was already held: a stale roster beats none, and
 * Fleet refuses an id that has gone, so the worst case is a picker offering one
 * value too many rather than a form with nothing in it.
 *
 */
export async function holdingsOf(port: number, held: Holdings, picked: Picked): Promise<Holdings> {
  const [workflows, manifests, models, leftOut] = await Promise.all([
    ask(port, "GET", "/workflows"),
    ask(port, "GET", "/manifests"),
    ask(port, "GET", "/models"),
    leftOutAsked(port, picked),
  ]);
  return {
    workflows: workflows.ok === true ? (workflows.body as WorkflowSummary[]) : held.workflows,
    manifests: manifests.ok === true ? (manifests.body as ManifestSummary[]) : held.manifests,
    models: models.ok === true ? (models.body as ModelChoices) : held.models,
    leftOut: leftOut === null ? [] : leftOut.ok === true ? (leftOut.body as LeftOutWorkflow[]) : held.leftOut,
    ...(held.repositories === undefined ? {} : { repositories: held.repositories }),
  };
}

/**
 * `/workflows/left_out`, named by the pick — `holdingsOf`'s one scoped field. `null` where a
 * repository not set up has no catalogue to have left anything out of.
 */
async function leftOutAsked(port: number, picked: Picked): Promise<Answer | null> {
  const path = picked.manifest("/workflows/left_out");
  return path === null ? null : await ask(port, "GET", path);
}

/** Every repository Fleet serves, set up or not. `null` where Fleet did not answer, which an older Fleet does not. */
export async function repositoriesOf(port: number): Promise<RepositoryList | null> {
  const answer = await ask(port, "GET", "/repositories");
  return answer.ok === true ? (answer.body as RepositoryList) : null;
}
/** Each served repository's merge line. `null` where Fleet did not answer. */
export async function mergeLinesOf(port: number): Promise<MergeLines | null> {
  const answer = await ask(port, "GET", "/merge_lines");
  return answer.ok === true ? (answer.body as MergeLines) : null;
}

/**
 * Fleet's three admission limits, and what Armada ships them at.
 *
 * **`null` where Fleet did not answer**: a failed read keeping the last figures would let the panel
 * draw values nobody can save over any more.
 */
export async function limitsOf(port: number): Promise<FleetLimits | null> {
  const answer = await ask(port, "GET", "/limits");
  return answer.ok === true ? (answer.body as FleetLimits) : null;
}

/**
 * Every preference a person has saved, and what is in force for each.
 *
 * **`null` where Fleet did not answer**, and the caller keeps whatever
 * `BridgeState.preferences` already holds — the shipped default until the
 * first read, and the last good reading after. Unlike `limitsOf`, a failed
 * read here is never taken as "nothing is saved": that would draw a person's
 * own choice as the default the moment a read glitches.
 */
export async function preferencesOf(port: number): Promise<Preferences | null> {
  const answer = await ask(port, "GET", "/preferences");
  return answer.ok === true ? (answer.body as Preferences) : null;
}

/**
 * What Fleet's last read of `armada.yml` came to.
 *
 * **Two `null`s that mean the same thing on screen and are not the same fact.**
 * Fleet answers `null` where it has not re-read the file since it started, and
 * this answers `null` where the read itself failed. Both draw nothing, which is
 * right — there is no reading to report either way — and neither is worth a
 * third state a surface would have to say something about.
 *
 * Asked once per connection rather than on a timer: the reading changes when
 * somebody saves a file, and `manifest.reread` is what says so. This is for the
 * window that opened *after* the save, which an event cannot reach.
 */
export async function manifestReadingOf(port: number, picked: Picked): Promise<ManifestReading | null> {
  const path = picked.manifest("/manifest/reading");
  if (path === null) return null;
  const answer = await ask(port, "GET", path);
  return answer.ok === true ? ((answer.body as ManifestReading | null) ?? null) : null;
}
/**
 * Every server Fleet holds — each Job's and the main checkout's — and the last
 * instance of each that ended.
 *
 * **`null` where Fleet did not answer**, a failed read keeps nothing: a server's
 * phase is live and a stale list would draw a *Serving* row for one that has
 * since exited. Read once per connection, like the capacity and the Manifest
 * reading beside it — after that, `server.*` on `/events` carries each row
 * whole, so nothing here is re-fetched on a timer.
 */
export async function serversOf(port: number): Promise<ServerList | null> {
  const answer = await ask(port, "GET", "/servers");
  return answer.ok === true ? (answer.body as ServerList) : null;
}

/**
 * One Check a Session's agent ran, read into the app by its run id.
 *
 * **A read that answers rather than one that is held.** A recorded output cannot move, it is asked
 * for by one reader about one Check, and it is a test runner's whole log. The refusal is carried
 * through whole.
 */
export async function sessionCheckOutputOf(port: number, run: number): Promise<CheckOutputRead> {
  const answer = await ask(port, "GET", `/sessions/checks/${encodeURIComponent(String(run))}/output`);
  if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
  return { ok: true, output: answer.body as CheckOutput };
}

/**
 * Ask a running Session Check to stop: `POST /sessions/checks/:run/stop`, or `/force_stop` for `SIGKILL`.
 * Fleet holds the ask for the run's own `armada check`, which carries it out; the run then ends as any does.
 */
export async function stopSessionCheck(port: number, run: number, force: boolean): Promise<Outcome> {
  const answer = await ask(port, "POST", `/sessions/checks/${encodeURIComponent(String(run))}/${force ? "force_stop" : "stop"}`);
  return answer.ok === true ? { ok: true } : answer.outcome;
}

/**
 * One Session's retro, read into the app: `GET /sessions/:session_id/retro?n=`. Asked when a surface opens it and again on focus, since nothing on `/events`
 * says a retro was written.
 */
export async function retroOf(port: number, subject: RetroSubject): Promise<RetroRead> {
  const path = `/sessions/${encodeURIComponent(subject.id)}/retro${subject.n === undefined ? "" : `?n=${Math.trunc(subject.n)}`}`;
  const answer = await ask(port, "GET", path);
  if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
  return { ok: true, retro: answer.body as Retro };
}

/**
 * The Lessons listing, `GET /lessons` (23.12): every repository on All and the pick's alone on a
 * pick — `Picked.narrowed`. A repository with no Manifest has no Jobs, so nothing is asked.
 */
export async function lessonsOf(
  port: number,
  picked: Picked,
  state: "open" | "accepted" = "open",
): Promise<LessonsRead> {
  const narrowed = picked.narrowed("/lessons");
  if (narrowed === null) return { ok: true, lessons: [] };
  // `open` is what Fleet answers where `state` is not sent, so it is not sent.
  const path = state === "open" ? narrowed : `${narrowed}${narrowed.includes("?") ? "&" : "?"}state=${state}`;
  const answer = await ask(port, "GET", path);
  if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
  return { ok: true, lessons: (answer.body as Lessons).lessons };
}

/**
 * The guided review, `POST /lessons/review`: the model reads every open item and orders them. A
 * model call, so it waits `MODEL_CALL_MS`. **The pick rides the body as `manifest_id`**, absent on
 * All; a repository with no Manifest has no items, and nothing is asked.
 */
export async function reviewOf(port: number, picked: Picked): Promise<LessonReviewRead> {
  const manifest = picked.narrowed("/lessons");
  if (manifest === null) return { ok: true, review: { model: "", entries: [], set_aside: [] } };
  const id = new URL(manifest, "http://fleet").searchParams.get("manifest_id");
  const answer = await ask(port, "POST", "/lessons/review", id === null ? {} : { manifest_id: id }, MODEL_CALL_MS);
  if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
  return { ok: true, review: answer.body as LessonReview };
}

/**
 * One question about one item, `POST /lessons/:lesson_id/ask`. **The thread is cut to what the
 * wire takes** here, because the renderer is sandboxed and its argument is not trusted: two roles,
 * text only, nothing else it may have attached.
 */
export async function askOf(port: number, lessonId: string, question: string, history: AskTurn[]): Promise<AskLessonRead> {
  const turns = Array.isArray(history)
    ? history.flatMap((turn) =>
        (turn?.role === "person" || turn?.role === "fleet") && typeof turn.text === "string"
          ? [{ role: turn.role, text: turn.text }]
          : [],
      )
    : [];
  const body: LessonAsk = { question: String(question), history: turns };
  const answer = await ask(port, "POST", `/lessons/${encodeURIComponent(lessonId)}/ask`, body, MODEL_CALL_MS);
  if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
  return { ok: true, answer: answer.body as AskLessonAnswer };
}

/**
 * One frame a step's harness produced, read into the app as the file it is.
 *
 * **The one read here that does not parse JSON**, and the reason is upstream:
 * an image has no window, so Fleet answers the file rather than an envelope
 * describing part of it. `ask` is not reused because its whole tail is
 * `JSON.parse` — what is shared is the refusal, which this asks it for by
 * reading the body twice over two paths rather than by branching inside it.
 *
 * **`kept` is two path segments and one id**, so it is split rather than
 * encoded whole: a frame's name is the harness's own and the run's directory in
 * front of it is what identifies a row. Nothing here composes it — Fleet handed
 * it over on the row, and this sends back what it was given.
 *
 * The bytes stop here on their way to a `Blob`. **The renderer never reaches
 * Fleet**, which is the rule every other read on this seam follows and the
 * reason a frame is fetched by main at all rather than put in an `img` tag
 * pointed at a port.
 */
export async function frameOf(port: number, jobId: string, kept: string): Promise<FrameRead> {
  const path = kept
    .split("/")
    .map((part) => encodeURIComponent(part))
    .join("/");
  return await fileAt(port, `/jobs/${jobId}/frames/${path}`);
}

/**
 * The picture one Note kept, read the same way — #1352. **One segment, because
 * a Studio keeps one frame per node**, and Fleet reads the file's name off the
 * node's own record, so nothing composed here reaches a path.
 */
export async function studioFrameOf(
  port: number,
  studioId: string,
  nodeId: string,
  picture?: string,
): Promise<FrameRead> {
  // A Sketch keeps one frame per picture, named by `?picture=` — since 20.0.
  const which = picture === undefined ? "" : `?picture=${encodeURIComponent(picture)}`;
  const at = `/studios/${encodeURIComponent(studioId)}/frames/${encodeURIComponent(nodeId)}${which}`;
  return await fileAt(port, at);
}

/** A picture or file a hosted session's message carried, read the way a frame is. `GET /sessions/file`. */
export async function sessionFileOf(port: number, sessionId: string, file: string): Promise<FrameRead> {
  return await fileAt(port, `/sessions/file?session_id=${encodeURIComponent(sessionId)}&file=${encodeURIComponent(file)}`);
}

/** The file at one path, as [`frameOf`] describes: bytes and a media type, or a refusal. */
async function fileAt(port: number, path: string): Promise<FrameRead> {
  const asked = { method: "GET" as const, path };
  try {
    const answer = await fetch(`http://${HOST}:${port}${asked.path}`, {
      method: "GET",
      headers: CALLER,
      signal: AbortSignal.timeout(FRAME_MS),
      dispatcher: FLEET,
    });
    if (!answer.ok) {
      // The refusal is JSON even though the success is not, so it is read as
      // text here and handed to the same parser every other route uses.
      return { ok: false, outcome: refusedWith(answer.status, await answer.text(), asked) };
    }
    return {
      ok: true,
      bytes: new Uint8Array(await answer.arrayBuffer()),
      // What Fleet said it is, never what this guesses. Fleet reads the media
      // type off the file's own name and sends `nosniff` with it; a second
      // opinion here would be the one that is wrong.
      type: answer.headers.get("content-type") ?? "application/octet-stream",
    };
  } catch (cause) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    const fault: TransportFault =
      cause instanceof Error && cause.name === "TimeoutError"
        ? { ...asked, why: "timed_out", waitedMs: FRAME_MS }
        : { ...asked, why: "unreachable" };
    return { ok: false, outcome: { ok: false, why: "transport", detail, fault } };
  }
}
