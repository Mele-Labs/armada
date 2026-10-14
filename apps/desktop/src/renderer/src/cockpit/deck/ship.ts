// The ship as the flight deck reads it: one list of subjects (a Job, a Session or a pull request nobody owns), each
// with the one phase it is in, the one mark that says how it stands and the pull request that is a phase of it; main
// per repository, the bays, Autopilot, and the calls. Read off what the window already holds (the Board, the Sessions,
// the merge line, the pool), and nothing here is a second vocabulary: a Session's state is its `standing` through
// `session-state.ts`. **Nothing is held locally.** A pull request is landed when the forge says so, a Session is
// stopped when Fleet says its turn is, and main is being fixed when the merge line says a Job is on it. #2301.

import { useEffect, useMemo, type ComponentProps } from "react";
import { Bot, CircleDashed, Eye, ShieldX, type LucideIcon } from "lucide-react";
import { sessionGlyphOf } from "@armada/components";
import type { HubJob, HubPullRequest, JobSummary, MergeLine, RepositorySummary, WorktreeSlot } from "@armada/protocol";
import { attachmentsOf, type Session, type SessionAttachment } from "@armada/screens/src/draft/sessions";
import type { NowView } from "@armada/jobs/draft/now";

import { useItems, type Hosts, type Item } from "../../Dashboard";
import { stateOf } from "../../session-state";
import { useSessions } from "../../sessions-draft";
import type { Cockpit } from "../Cockpit";
import { sessionIdOf } from "../waiting";

type CockpitProps = ComponentProps<typeof Cockpit>;
/** What the ship reads: the window's state and the hosts `useItems` raises its calls through, so the title bar can read it without the Cockpit's other props. */
export type ShipProps = Pick<CockpitProps, "state" | "now" | "picked" | "nowViews" | "nows" | "onWantHeld"> & Hosts;

/** Where a subject stands in the one pipeline the strips board draws as columns. */
/** Where a Job or Session stands, whatever path it is on: the words the timeline's groups use. */
export type Phase = "needs" | "working" | "held" | "done";
export const PHASES: readonly Phase[] = ["needs", "working", "held", "done"];

/** Gray is calm. `ask` is waiting on him; `bad` is broken. Colour and motion are only for those two. */
export type Tone = "calm" | "ask" | "bad";

export type Mark = { Glyph: LucideIcon; said: string; tone: Tone };

/** A pull request, a phase of whoever owns it. */
export type Pr = {
  key: string;
  root: string;
  number: number;
  title: string;
  branch: string;
  url: string;
  ci: "passed" | "running" | "failed" | "waiting_on_main";
  queue?: { state: string; position?: number };
  author?: string;
  stage: "checking" | "red" | "queued" | "ready" | "landed";
  owner?: { kind: "job" | "session"; id: string; title: string };
};

export type Subject = {
  key: string;
  kind: "job" | "session" | "pr";
  id: string;
  title: string;
  root: string;
  repo: string;
  mark: Mark;
  phase: Phase;
  /** He is owed something: a question, a reply, a done, a review, a failed Check. */
  needs: boolean;
  /** Something is broken: stuck, red. */
  alarm: boolean;
  /** The call that is about it, where one is raised. */
  call?: string;
  pr?: Pr;
  slot?: number;
  step?: { at: number; of: number; label: string };
  /** The last lines of what it is doing, for Peek. */
  peek: string[];
  /** A Session's turn is open, so Stop applies. A Job that is not over. */
  stoppable: boolean;
  repliable: boolean;
  /** Its pull request can be merged now. */
  mergeable: boolean;
  /** Minutes since it last said anything, where it is a Session. */
  since?: string;
};

export type Main = {
  root: string;
  repo: string;
  state: "green" | "red" | "running" | "unknown";
  check?: string;
  test?: string;
  broke?: { number: number; title: string };
  fixing: boolean;
  /** Who merged the pull request that turned it red. */
  by?: { kind: "job" | "session"; id: string; title: string };
};

export type Bay = {
  root: string;
  repo: string;
  slot: number;
  state: "free" | "held" | "closed";
  holder?: { kind: "job" | "session"; id: string; title: string };
  branch?: string;
};

export type Ship = {
  subjects: readonly Subject[];
  mains: readonly Main[];
  /** The one main the ship has: red if any is. */
  main: Main | undefined;
  bays: readonly Bay[];
  autopilot: "off" | "running" | "paused";
  /** Every call Fleet raises, oldest first. */
  calls: readonly Item[];
  prs: readonly Pr[];
  repos: readonly { root: string; name: string }[];
  /** The Jobs and Sessions with a Check running now: a Job's own, or one on the pull request a Job or Session holds. */
  running: ReadonlySet<string>;
};

const NONE: ReadonlySet<string> = new Set();
const one = (text: string) => text.split("\n")[0]!.trim();

function sessionMark(session: Session): Mark {
  const { state, said } = stateOf(session);
  const tone: Tone = state === "waiting" || state === "replied" || state === "done" ? "ask" : state === "stuck" || state === "failing" ? "bad" : "calm";
  return { Glyph: sessionGlyphOf(state), said, tone };
}

function jobMark(job: JobSummary): Mark {
  if (job.asking === true) return { Glyph: ShieldX, said: "Check failed", tone: "bad" };
  if (job.status === "awaiting_review") return { Glyph: Eye, said: "Awaiting review", tone: "ask" };
  if (job.status === "queued") return { Glyph: CircleDashed, said: "Queued", tone: "calm" };
  return { Glyph: Bot, said: "Drone working", tone: "calm" };
}

function peekOfSession(session: Session): string[] {
  return session.rows
    .filter((row) => row.kind === "message" || row.kind === "tool" || row.kind === "check")
    .slice(-3)
    .map((row) => (row.kind === "check" ? `${row.name} ${row.state}` : one(row.text)));
}

function peekOfJob(view: NowView | undefined): string[] {
  const drone = view?.running?.find((one) => one.of === "drone");
  const lines = drone?.tail ?? (drone?.line === undefined ? [] : [drone.line]);
  const checks = (view?.running ?? []).filter((one) => one.of === "check").map((one) => `${one.name} ${one.state}`);
  return [...lines.slice(-2), ...checks].slice(-3);
}

const ageOf = (iso: string | undefined, now: number) => (iso === undefined ? undefined : `${Math.max(0, Math.round((now - Date.parse(iso)) / 60_000))}m`);

/** The stage a pull request is at, from what the forge says. */
function stageOf(pull: Pick<HubPullRequest, "ci" | "queue">): Pr["stage"] {
  if (pull.ci === "failed") return "red";
  const q = pull.queue;
  if (q !== undefined && (q.position !== undefined || ["queued", "awaiting_checks", "mergeable"].includes(q.state))) return "queued";
  if (pull.ci === "passed") return "ready";
  return "checking";
}

const ciOf = (ci: string | undefined): Pr["ci"] => (ci === "passed" || ci === "failed" || ci === "waiting_on_main" ? ci : "running");

type Inputs = {
  jobs: readonly JobSummary[];
  sessions: readonly Session[];
  lines: readonly MergeLine[];
  slots: readonly WorktreeSlot[];
  repositories: readonly RepositorySummary[];
  workflows: readonly { id: string; steps: readonly { step_id: string; label: string }[] }[];
  nows: Readonly<Record<string, NowView>> | undefined;
  calls: readonly Item[];
  now: number;
};

/** Pure: the ship from what the window holds. */
export function shipOf(input: Inputs): Ship {
  const { jobs, sessions, lines, slots, repositories, workflows, nows, calls, now } = input;
  const rootOf = (manifest: string | undefined) => repositories.find((one) => one.manifest?.id === manifest)?.root ?? repositories[0]?.root ?? "";
  const nameOfRoot = (root: string) => repositories.find((one) => one.root === root)?.manifest?.repository ?? root;
  const repos = lines.map((line) => ({ root: line.root, name: nameOfRoot(line.root) }));

  // Pull requests, every one the forge lists, open first and then the newest landed.
  const prs: Pr[] = [];
  const owners = {
    byJob: (pull: { number: number; branch: string; job?: HubJob }) => (pull.job ?? jobs.filter((job) => job.branch === pull.branch).map((job) => ({ id: job.id, title: job.title }))[0]),
    bySession: (pull: { number: number; branch: string }) =>
      sessions.find((one) => one.dead === undefined && one.attachments.some((a) => (a.kind === "pull_request" && (a.number === pull.number || a.branch === pull.branch)) || (a.kind === "branch" && a.name === pull.branch))),
  };
  const ownerOf = (pull: { number: number; branch: string; job?: HubJob }): Pr["owner"] => {
    const job = owners.byJob(pull);
    if (job !== undefined) return { kind: "job", id: job.id, title: job.title };
    const session = owners.bySession(pull);
    return session === undefined ? undefined : { kind: "session", id: session.id, title: session.title ?? session.id };
  };
  for (const line of lines) {
    for (const pull of line.hub?.pull_requests ?? []) {
      const owner = ownerOf(pull);
      prs.push({
        key: `pr:${line.root}:${pull.number}`,
        root: line.root,
        number: pull.number,
        title: pull.title,
        branch: pull.branch,
        url: pull.url,
        ci: ciOf(pull.ci),
        ...(pull.queue === undefined ? {} : { queue: pull.queue }),
        ...(pull.author === undefined ? {} : { author: pull.author }),
        stage: stageOf(pull),
        ...(owner === undefined ? {} : { owner }),
      });
    }
    for (const pull of (line.hub?.merged ?? []).slice(0, 3)) {
      if (prs.some((one) => one.number === pull.number && one.root === line.root)) continue;
      const owner = ownerOf(pull);
      prs.push({ key: `pr:${line.root}:${pull.number}`, root: line.root, number: pull.number, title: pull.title, branch: pull.branch, url: pull.url, ci: "passed", stage: "landed", ...(owner === undefined ? {} : { owner }) });
    }
  }
  const prOf = (owner: { kind: "job" | "session"; id: string }) => prs.find((one) => one.owner?.kind === owner.kind && one.owner.id === owner.id && one.stage !== "landed") ?? prs.find((one) => one.owner?.kind === owner.kind && one.owner.id === owner.id);
  const callOf = (find: (item: Item) => boolean) => calls.find(find)?.key;

  const subjects: Subject[] = [];
  for (const job of jobs) {
    if (/completed|killed|rejected|superseded|failed/.test(job.status)) continue;
    const mark = jobMark(job);
    const pr = prOf({ kind: "job", id: job.id });
    const steps = workflows.find((one) => one.id === job.workflow_id)?.steps ?? [];
    const at = steps.findIndex((step) => step.step_id === job.current_step_id);
    const root = rootOf(job.owner_manifest_id);
    const needs = mark.tone === "ask" || mark.tone === "bad";
    subjects.push({
      key: `job:${job.id}`,
      kind: "job",
      id: job.id,
      title: job.title,
      root,
      repo: nameOfRoot(root),
      mark,
      phase: needs ? "needs" : job.status === "queued" ? "held" : "working",
      needs,
      alarm: false,
      ...(callOf((item) => item.job?.id === job.id) === undefined ? {} : { call: callOf((item) => item.job?.id === job.id)! }),
      ...(pr === undefined ? {} : { pr }),
      ...(at < 0 ? {} : { step: { at: at + 1, of: steps.length, label: steps[at]!.label } }),
      peek: peekOfJob(nows?.[job.id]),
      stoppable: job.status !== "queued",
      repliable: false,
      mergeable: pr !== undefined && (pr.stage === "queued" || pr.stage === "ready") && pr.ci === "passed",
    });
  }

  for (const session of sessions) {
    if (session.dead !== undefined || session.helm === true) continue;
    const mark = sessionMark(session);
    const own = attachmentsOf(session, "pull_request").find((one) => one.state !== "merged") ?? attachmentsOf(session, "pull_request")[0];
    const pr = prOf({ kind: "session", id: session.id }) ?? (own === undefined ? undefined : synth(own, rootOf(session.manifestId)));
    const needs = mark.tone === "ask";
    const stuck = stateOf(session).state === "stuck";
    const redPr = pr !== undefined && pr.stage === "red";
    const idle = session.turn.state === "idle";
    const alarm = mark.tone === "bad" || (redPr && idle);
    const slot = slots.find((one) => one.held.state === "hosted_session" && one.held.session_id === session.id)?.slot ?? attachmentsOf(session, "branch")[0]?.slot;
    const root = rootOf(session.manifestId);
    const call = callOf((item) => sessionIdOf(item.key) === session.id);
    subjects.push({
      key: `session:${session.id}`,
      kind: "session",
      id: session.id,
      title: session.title ?? session.id,
      root,
      repo: nameOfRoot(root),
      mark,
      phase: needs || (alarm && !stuck) ? "needs" : stuck ? "held" : session.turn.state === "working" ? "working" : "done",
      needs,
      alarm,
      ...(call === undefined ? {} : { call }),
      ...(pr === undefined ? {} : { pr }),
      ...(slot === undefined ? {} : { slot }),
      peek: peekOfSession(session),
      stoppable: session.turn.state === "working",
      repliable: true,
      mergeable: pr !== undefined && (pr.stage === "queued" || pr.stage === "ready") && pr.ci === "passed",
      ...(ageOf(session.lastTurnAt, now) === undefined ? {} : { since: ageOf(session.lastTurnAt, now)! }),
    });
  }

  // A pull request nobody in Armada owns is a strip of its own.
  const owned = new Set(subjects.flatMap((one) => (one.pr === undefined ? [] : [one.pr.key])));
  for (const pr of prs) {
    if (owned.has(pr.key)) continue;
    const mergeable = pr.stage === "queued" || pr.stage === "ready";
    const mark: Mark = { Glyph: sessionGlyphOf("idle"), said: pr.stage === "red" ? "Checks failed" : pr.stage === "landed" ? "Merged" : pr.stage === "ready" ? "Ready" : pr.stage === "queued" ? "In the queue" : "Checks running", tone: pr.stage === "red" ? "bad" : "calm" };
    subjects.push({
      key: pr.key,
      kind: "pr",
      id: String(pr.number),
      title: pr.title,
      root: pr.root,
      repo: nameOfRoot(pr.root),
      mark,
      phase: pr.stage === "red" ? "needs" : "done",
      needs: false,
      alarm: pr.stage === "red",
      pr,
      peek: [`#${pr.number} ${pr.title}`, pr.branch, pr.author === undefined ? "" : `by ${pr.author}`].filter((line) => line !== ""),
      stoppable: false,
      repliable: false,
      mergeable: mergeable && pr.ci === "passed",
    });
  }

  const mains: Main[] = lines.flatMap((line): Main[] => {
    const main = line.hub?.main;
    if (main === undefined) return [];
    const red = main.state === "red";
    const first = main.failed?.[0];
    const merge = main.merge;
    const fixing = red && line.hub?.fixing !== undefined;
    return [
      {
        root: line.root,
        repo: nameOfRoot(line.root),
        state: red ? "red" : (main.checking ?? []).length > 0 ? "running" : main.state === "green" ? "green" : "unknown",
        ...(first === undefined ? {} : { check: first.check ?? first.name }),
        ...(first?.tests?.[0] === undefined ? {} : { test: first.tests[0] }),
        ...(merge === undefined ? {} : { broke: { number: merge.number, title: prs.find((one) => one.number === merge.number)?.title ?? merge.branch ?? "" } }),
        ...(merge?.session === undefined ? merge?.job === undefined ? {} : { by: { kind: "job" as const, id: merge.job.id, title: merge.job.title } } : { by: { kind: "session" as const, id: merge.session.id, title: merge.session.title ?? merge.session.id } }),
        fixing,
      },
    ];
  });
  const main = mains.find((one) => one.state === "red") ?? mains[0];

  const bays: Bay[] = slots.map((slot) => {
    const held = slot.held;
    const holder: Bay["holder"] =
      held.state === "job" ? { kind: "job", id: held.job_id, title: held.job_title ?? held.job_id } : held.state === "hosted_session" ? { kind: "session", id: held.session_id, title: held.session_title ?? held.session_id } : undefined;
    return {
      root: rootOf(slot.manifest_id),
      repo: nameOfRoot(rootOf(slot.manifest_id)),
      slot: slot.slot,
      state: slot.closed === true ? "closed" : holder === undefined ? "free" : "held",
      ...(holder === undefined ? {} : { holder }),
      ...(slot.branch === undefined ? {} : { branch: slot.branch }),
    };
  });

  const running = new Set<string>();
  for (const job of jobs) if ((nows?.[job.id]?.running ?? []).some((one) => one.of === "check" && one.state === "running")) running.add(`job:${job.id}`);
  for (const pr of prs) if (pr.stage === "checking" && pr.owner !== undefined) running.add(`${pr.owner.kind}:${pr.owner.id}`);
  // A Session counts for the Jobs it owns.
  for (const session of sessions) if (session.attachments.some((a) => a.kind === "job" && running.has(`job:${a.id}`))) running.add(`session:${session.id}`);

  return { subjects, mains, main, bays, autopilot: "off", calls, prs, repos, running };
}

/** A pull request a Session holds that the forge's listing does not carry any more. */
function synth(own: Extract<SessionAttachment, { kind: "pull_request" }>, root: string): Pr {
  const merged = own.state === "merged";
  return {
    key: `pr:${root}:${own.number}`,
    root,
    number: own.number,
    title: own.title,
    branch: own.branch,
    url: own.address,
    ci: own.checks.state === "failed" ? "failed" : own.checks.state === "passed" ? "passed" : "running",
    stage: merged ? "landed" : own.checks.state === "failed" ? "red" : own.queued === true ? "queued" : own.checks.state === "passed" ? "ready" : "checking",
  };
}

const slotsOf = (state: CockpitProps["state"], picked: CockpitProps["picked"]): readonly WorktreeSlot[] =>
  state.held.state === "read" ? (state.held.held.slots ?? []).filter((one) => picked?.manifest === undefined || one.manifest_id === picked.manifest.id) : [];

/** The ship, live. `hosts` is what `useItems` raises calls through. */
export function useShip(props: ShipProps, hold = true): Ship {
  const { state, now, picked, nowViews, nows } = props;
  const hosts: Hosts = props;
  const sessions = useSessions();
  // Fleet serves the worktree pool only while someone draws the bays, and these bodies draw them.
  const { onWantHeld } = props;
  useEffect(() => {
    if (!hold) return;
    onWantHeld?.(true);
    return () => onWantHeld?.(false);
  }, [onWantHeld, hold]);
  const calls = useItems("command-central", state, picked, nowViews, hosts, NONE);
  return useMemo(
    () =>
      shipOf({
        jobs: state.jobs,
        sessions,
        lines: state.mergeLines?.lines ?? [],
        slots: slotsOf(state, picked),
        repositories: state.holds.repositories ?? [],
        workflows: state.holds.workflows,
        nows,
        calls,
        now,
      }),
    [state, sessions, picked, nows, calls, now],
  );
}
