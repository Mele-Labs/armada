// The ship as the flight deck reads it: one list of subjects (a Session or a pull request nobody owns), each
// with the one phase it is in, the one mark that says how it stands and the pull request that is a phase of it; main
// per repository, the bays, Autopilot, and the calls. Read off what the window already holds (the Sessions, the merge
// line, the pool), and nothing here is a second vocabulary: a Session's state is its `standing` through
// `session-state.ts`. **Nothing is held locally.** A pull request is landed when the forge says so, a Session is
// stopped when Fleet says its turn is, and main is being fixed when the merge line says a Session is on it. #2301.

import { useEffect, useMemo } from "react";
import type { LucideIcon } from "lucide-react";
import { sessionGlyphOf } from "@armada/components";
import type { HubPullRequest, MergeLine, RepositorySummary, WorktreeSlot } from "@armada/protocol";
import { attachmentsOf, type Session, type SessionAttachment } from "@armada/screens/src/draft/sessions";

import { useItems, type Hosts, type Item } from "../../Dashboard";
import { accountNowOf } from "../../session-activity";
import { stateOf } from "../../session-state";
import { useSessions } from "../../sessions-draft";
import { useViewedVersion } from "../viewed";
import type { CockpitProps } from "./props";
import { callKindOf, sessionIdOf, waitingOf } from "../waiting";
import { jobIdOf } from "@armada/shell";

/** What the ship reads: the window's state and the hosts `useItems` raises its calls through, so the title bar can read it without the Cockpit's other props. */
export type ShipProps = Pick<CockpitProps, "state" | "now" | "picked" | "onWantHeld"> & Hosts;

/** Where a subject stands in the one pipeline the strips board draws as columns. */
/** Where a Session stands, whatever path it is on: the words the timeline's groups use. */
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
  owner?: { kind: "session"; id: string; title: string };
};

export type Subject = {
  key: string;
  kind: "session" | "pr";
  id: string;
  title: string;
  root: string;
  repo: string;
  /** The Manifest id it was dispatched under, where it carries one. The list's repository headings read it, as `root` falls back to the first repository. */
  manifest?: string;
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
  /** The last lines of what it is doing, for Peek. */
  peek: string[];
  /** The agent's last reply only told him something and he has not opened the Session since. It is calm and raises no call, and the list draws it with a quiet mark; a reply that asks is a call instead. */
  reply?: true;
  /** A Session's turn is open, so Stop applies. */
  stoppable: boolean;
  repliable: boolean;
  /** Its pull request can be merged now. */
  mergeable: boolean;
  /** Whole minutes since it last said anything, where it is a Session: a failure left that long is drawn quiet. */
  idleMinutes?: number;
  /**
   * A Session that ended, or that Fleet cannot reach. It is a row of the list under Quiet, Ended or All, and nothing else of the
   * deck's: it flies no drone, raises no call and holds no bay.
   */
  past?: "quiet" | "ended";
  /** An ended Session last seen before the list's week: the list leaves it off until a search finds it. */
  older?: true;
  /** When it last took a turn, in ms: the order a past list reads in, and the age the row draws. */
  at?: number;
  /** A Session another Session started: that Session's key, and the title of the plan group it was handed where it was handed one. The list draws it under its parent. */
  linked?: { parent: string; group?: string };
  /** The Job this one took the place of, by id: the row offers it, and the order gives this one that Job's place. */
  earlier?: string;
  /** A Job another took the place of: it is no row of any list, and opens only from that Job's row. */
  behind?: true;
  /** This Job asked to carry on in a new one and was told yes: the new one is coming up, or did not come. */
  handoff?: "starting" | "failed";
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
  by?: { kind: "session"; id: string; title: string };
};

export type Bay = {
  root: string;
  repo: string;
  slot: number;
  state: "free" | "held" | "closed";
  holder?: { kind: "session"; id: string; title: string };
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
  /** The Sessions with a Check running now, on the pull request a Session holds. */
  running: ReadonlySet<string>;
  /** The Sessions waiting on an answer from the person: a question, a waiting item, a gate. Their call stays until answered. */
  asking: ReadonlySet<string>;
};

const NONE: ReadonlySet<string> = new Set();
const one = (text: string) => text.split("\n")[0]!.trim();

function sessionMark(session: Session): Mark {
  const { state, said } = stateOf(session);
  const tone: Tone = state === "waiting" || state === "done" || state === "draft" ? "ask" : state === "stuck" || state === "failing" ? "bad" : "calm";
  return { Glyph: sessionGlyphOf(state), said, tone };
}

/** The row's line under the title is the last of these: the account's now line while a turn runs, else what it last said. */
export function peekOfSession(session: Session): string[] {
  const now = accountNowOf(session);
  return [
    ...session.rows
      .filter((row) => row.kind === "message" || row.kind === "tool" || row.kind === "check")
      .slice(-3)
      .map((row) => (row.kind === "check" ? `${row.name} ${row.state}` : one(row.text))),
    ...(now === undefined ? [] : [now]),
  ];
}


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
  sessions: readonly Session[];
  lines: readonly MergeLine[];
  slots: readonly WorktreeSlot[];
  repositories: readonly RepositorySummary[];
  calls: readonly Item[];
  now: number;
};

/** Pure: the ship from what the window holds. */
export function shipOf(input: Inputs): Ship {
  const { sessions, lines, slots, repositories, calls, now } = input;
  const rootOf = (manifest: string | undefined) => repositories.find((one) => jobIdOf(one) === manifest)?.root ?? repositories[0]?.root ?? "";
  const nameOfRoot = (root: string) => repositories.find((one) => one.root === root)?.manifest?.repository ?? root;
  const repos = lines.map((line) => ({ root: line.root, name: nameOfRoot(line.root) }));

  // Pull requests, every one the forge lists, open first and then the newest landed.
  const prs: Pr[] = [];
  const ownerOf = (pull: { number: number; branch: string }): Pr["owner"] => {
    const session = sessions.find((one) => one.dead === undefined && one.attachments.some((a) => (a.kind === "pull_request" && (a.number === pull.number || a.branch === pull.branch)) || (a.kind === "branch" && a.name === pull.branch)));
    return session === undefined ? undefined : { kind: "session", id: session.id, title: session.title ?? session.id };
  };
  for (const line of lines) {
    for (const pull of line.hub?.pull_requests ?? []) {
      // Another author's is not Armada's to follow: no subject, no call, and its chip is drawn quiet.
      if (pull.mine === false) continue;
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
  const prOf = (owner: { id: string }) => prs.find((one) => one.owner?.id === owner.id && one.stage !== "landed") ?? prs.find((one) => one.owner?.id === owner.id);
  const callOf = (find: (item: Item) => boolean) => calls.find(find)?.key;

  const subjects: Subject[] = [];
  for (const session of sessions) {
    if (session.helm === true) continue;
    if (session.dead !== undefined) {
      const root = rootOf(session.manifestId);
      const at = session.lastTurnAt === undefined ? undefined : Date.parse(session.lastTurnAt);
      subjects.push({
        key: `session:${session.id}`,
        kind: "session",
        id: session.id,
        title: session.title ?? session.id,
        root,
        repo: nameOfRoot(root),
        ...(session.manifestId === undefined ? {} : { manifest: session.manifestId }),
        mark: sessionMark(session),
        phase: "done",
        needs: false,
        alarm: false,
        peek: peekOfSession(session),
        stoppable: false,
        repliable: false,
        mergeable: false,
        past: session.dead,
        ...(session.replacedBy === undefined ? {} : { behind: true as const }),
        ...(session.older === true ? { older: true as const } : {}),
        ...(at === undefined || Number.isNaN(at) ? {} : { at }),
      });
      continue;
    }
    const mark = sessionMark(session);
    const lastTurn = session.lastTurnAt === undefined ? undefined : Date.parse(session.lastTurnAt);
    const own = attachmentsOf(session, "pull_request").find((one) => one.state !== "merged") ?? attachmentsOf(session, "pull_request")[0];
    const pr = prOf({ id: session.id }) ?? (own === undefined ? undefined : synth(own, rootOf(session.manifestId)));
    const needs = mark.tone === "ask";
    const stuck = stateOf(session).state === "stuck";
    const redPr = pr !== undefined && pr.stage === "red";
    const idle = session.turn.state === "idle";
    const alarm = mark.tone === "bad" || (redPr && idle);
    const slot = slots.find((one) => one.held.state === "hosted_session" && one.held.session_id === session.id)?.slot ?? attachmentsOf(session, "branch")[0]?.slot;
    const root = rootOf(session.manifestId);
    const call = callOf((item) => sessionIdOf(item.key) === session.id);
    const handed = session.attachments.find((one): one is Extract<SessionAttachment, { kind: "started_by" }> => one.kind === "started_by");
    const parent = handed === undefined ? undefined : sessions.find((one) => one.id === handed.id);
    subjects.push({
      key: `session:${session.id}`,
      kind: "session",
      id: session.id,
      title: session.title ?? session.id,
      root,
      repo: nameOfRoot(root),
      ...(session.manifestId === undefined ? {} : { manifest: session.manifestId }),
      mark,
      phase: needs || (alarm && !stuck) ? "needs" : stuck || session.waits !== undefined ? "held" : stateOf(session).state === "working" ? "working" : "done",
      needs,
      alarm,
      ...(stateOf(session).state === "replied" ? { reply: true as const } : {}),
      ...(call === undefined ? {} : { call }),
      ...(pr === undefined ? {} : { pr }),
      ...(slot === undefined ? {} : { slot }),
      peek: peekOfSession(session),
      stoppable: session.turn.state === "working",
      repliable: session.waits === undefined,
      mergeable: pr !== undefined && (pr.stage === "queued" || pr.stage === "ready") && pr.ci === "passed",
      ...(lastTurn === undefined || Number.isNaN(lastTurn) ? {} : { at: lastTurn, idleMinutes: Math.max(0, Math.round((now - lastTurn) / 60_000)) }),
      ...(session.replaces === undefined ? {} : { earlier: session.replaces }),
      ...(session.handoff === undefined ? {} : { handoff: session.handoff }),
      ...(handed === undefined || parent === undefined ? {} : { linked: { parent: `session:${parent.id}`, ...(handed.group === undefined ? {} : { group: parent.work?.plan?.groups.find((one) => one.id === handed.group)?.title ?? handed.group }) } }),
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
    const fixing = red && line.hub?.fixing_session !== undefined;
    return [
      {
        root: line.root,
        repo: nameOfRoot(line.root),
        state: red ? "red" : (main.checking ?? []).length > 0 ? "running" : main.state === "green" ? "green" : "unknown",
        ...(first === undefined ? {} : { check: first.check ?? first.name }),
        ...(first?.tests?.[0] === undefined ? {} : { test: first.tests[0] }),
        ...(merge === undefined ? {} : { broke: { number: merge.number, title: prs.find((one) => one.number === merge.number)?.title ?? merge.branch ?? "" } }),
        ...(merge?.session === undefined ? {} : { by: { kind: "session" as const, id: merge.session.id, title: merge.session.title ?? merge.session.id } }),
        fixing,
      },
    ];
  });
  const main = mains.find((one) => one.state === "red") ?? mains[0];

  const bays: Bay[] = slots.map((slot) => {
    const held = slot.held;
    const holder: Bay["holder"] = held.state === "hosted_session" ? { kind: "session", id: held.session_id, title: held.session_title ?? held.session_id } : undefined;
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
  for (const pr of prs) if (pr.stage === "checking" && pr.owner !== undefined) running.add(`${pr.owner.kind}:${pr.owner.id}`);

  // A Session that needs the person is a call, whether or not it raised one: its row's mark, opened as the row is. One that
  // asks (a question, a waiting item, a gate) keeps it until answered; a done turn or a stuck one is a call until seen. A reply that only tells is none.
  // The key carries the turn, so the next reply is a new call.
  const asking = new Set(subjects.filter((one) => one.kind === "session" && stateOf(sessions.find((s) => s.id === one.id)!).state === "waiting").map((one) => one.key));
  const raised = [...calls];
  for (const subject of subjects) {
    if (subject.kind === "pr" || subject.phase !== "needs" || subject.call !== undefined) continue;
    const session = subject.kind === "session" ? sessions.find((one) => one.id === subject.id) : undefined;
    const at = session?.lastTurnAt;
    // What the Session waits on keeps the label its own call had before he looked, so the call returns as it was.
    const waits = session === undefined ? undefined : waitingOf(session).sort((a, b) => Date.parse(b.since) - Date.parse(a.since))[0];
    const item: Item = {
      key: `${subject.kind}:${subject.id}:needs:${at ?? ""}`,
      icon: subject.mark.Glyph,
      kind: waits === undefined ? subject.mark.said : callKindOf(waits.source),
      title: subject.title,
      fact: subject.mark.said,
      at,
      hue: "ask",
      where: subject.repo ?? "",
      body: [],
      acts: () => null,
    };
    raised.push(item);
    subject.call = item.key;
  }
  return { subjects, mains, main, bays, autopilot: "off", calls: raised, prs, repos, running, asking };
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

export const slotsOf = (state: CockpitProps["state"], picked: CockpitProps["picked"]): readonly WorktreeSlot[] =>
  state.held.state === "read" ? (state.held.held.slots ?? []).filter((one) => picked === null || jobIdOf(picked) === undefined || one.manifest_id === jobIdOf(picked)) : [];

/** The ship, live. `hosts` is what `useItems` raises calls through. */
export function useShip(props: ShipProps, hold = true): Ship {
  const { state, now, picked } = props;
  const hosts: Hosts = props;
  const sessions = useSessions();
  // Fleet serves the worktree pool only while someone draws the bays, and these bodies draw them.
  const { onWantHeld } = props;
  useEffect(() => {
    if (!hold) return;
    onWantHeld?.(true);
    return () => onWantHeld?.(false);
  }, [onWantHeld, hold]);
  const calls = useItems("command-central", state, picked, hosts, NONE);
  // A Session read is one that no longer needs him: build again when any Session's time does.
  const viewed = useViewedVersion();
  return useMemo(
    () =>
      shipOf({
        sessions,
        lines: state.mergeLines?.lines ?? [],
        slots: slotsOf(state, picked),
        repositories: state.holds.repositories ?? [],
        calls,
        now,
      }),
    [state, sessions, picked, calls, now, viewed],
  );
}
