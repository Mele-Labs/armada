// What the Dashboard's panel reads: every Job, Session and merge-line item as one `Item`, cut three
// ways — what needs the owner, what is under way, what is over — with the keys the Board's lists
// answer. `cockpit/` draws them. Mock only: what a Job asks comes from the draft's Now views, which
// Fleet does not serve.

import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Bot,
  Box,
  Check,
  CircleCheck,
  CircleDot,
  CircleX,
  Clock,
  Cpu,
  Eye,
  GitMerge,
  GitPullRequest,
  LoaderCircle,
  Megaphone,
  MessageSquare,
  OctagonAlert,
  Scale,
  ShieldCheck,
  ShieldX,
  SquareTerminal,
  Unplug,
  Waypoints,
  Workflow,
  type LucideIcon,
} from "lucide-react";
import { Button, Radio, RadioGroup, Textarea } from "@armada/components";
import type { FixMain, JobSummary, RepositorySummary, WaitingItem } from "@armada/protocol";
import type { AboutFiles, AboutLink, CallAskView, CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import type { Session } from "@armada/screens/src/draft/sessions";
import { overviewListsOf, type DashboardTab } from "@armada/overview";
import { JobActs, isTerminal, titleOf, type PauseAct } from "@armada/screens";
import { boardPressOf } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import type { BridgeState } from "../../shared/bridge";
import { viewsOf } from "./merge-line";
import { useSessions } from "./sessions-draft";
import { forgetCleared, identityOf, useDismissed } from "./cockpit/dismissed";
import { callsFromWaiting, sessionIdOf, type WaitingCall } from "./cockpit/waiting";

/** The frame's hue: what the row wants of the owner, or how it stands. */
type Hue = "ask" | "issue" | "running" | "queued" | "ok" | "bad";

export type Item = {
  key: string;
  /** The lane it lights on the fleet board: a Job's id, `session:<id>`, or `line`. */
  owner?: string;
  /** The Plan decisions, where the item is one: the call answers them as tiles. */
  decisions?: Extract<CallAskView, { kind: "plan" }>["decisions"];
  icon: LucideIcon;
  /** The icon's tooltip: what kind of thing the row is. */
  kind: string;
  title: string;
  fact: string;
  at?: string | undefined;
  hue: Hue;
  live?: boolean;
  /** The detail's eyebrow: where it is. */
  where: string;
  body: readonly (readonly [string, string])[];
  /** What it says, in its own words, under the facts. */
  said?: readonly string[];
  /** The Job it belongs to, where it is one: the call draws its workflow's steps from it. */
  job?: JobSummary;
  /** What the Job was dispatched with. */
  request?: string;
  /** Where the Job lives: repository, the code it touches, where it came from, its branch. */
  about?: readonly (readonly [string, string | AboutLink | AboutFiles])[];
  /** What it is doing, for a thing with no steps. */
  doing?: string;
  /** The words leading up to it, or the output it failed with, and what to call them. */
  context?: readonly string[];
  contextHead?: string;
  acts: (done: () => void) => ReactNode;
  /** The tile's state icon where the kind's own would not say what is active: a Session's state, or a Job's Drone, Check or Judge. */
  mark?: LucideIcon;
  /** What that icon's tooltip says. The kind where absent. */
  state?: string;
  /** Whether the item is a Session. */
  session?: boolean;
  /** The Job's workflow steps and the index it is at, for the tile's pips. */
  steps?: readonly { id: string; label: string }[];
  stepAt?: number;
  /** A Session's recent cadence, each bar 0 to 1: a message tall, a tool call short. */
  spark?: readonly number[];
  /** What a Session's call waits on: the item, which is what the call answers. */
  waiting?: { sessionId: string; item: WaitingItem };
};

/** What the Dashboard knows of each Job's Now, by Job id. Mock only: Fleet serves none. */
export const Nows = createContext<Readonly<Record<string, NowView>> | undefined>(undefined);

const WAITING = {
  resource: { mark: Cpu, state: "Waiting on a resource" },
  job: { mark: Box, state: "Waiting on a Job" },
  transition: { mark: Waypoints, state: "Between steps" },
  step: { mark: Workflow, state: "Running a one-off step" },
} as const;

/**
 * The first thing active on a Job, as the icon its tile wears: what asks the owner, then what went
 * wrong, then what runs (a Drone, a Check, a Judge), then what it waits on.
 */
function activeOf(view: NowView | undefined): Pick<Item, "mark" | "state" | "hue"> | undefined {
  if (view === undefined) return undefined;
  if ((view.asks ?? []).length > 0) return { mark: Megaphone, state: "Waiting on you", hue: "ask" };
  const issue = view.issues?.[0];
  if (issue !== undefined) return { mark: OctagonAlert, state: issue.said, hue: "issue" };
  const running = view.running?.find((one) => one.state === "running") ?? view.running?.[0];
  if (running !== undefined) {
    const said = running.state === "failed" ? "Check failed" : running.state === "passed" ? "Check passed" : running.of === "drone" ? "Drone working" : running.of === "check" ? "Check running" : "Judge deciding";
    return { mark: running.state === "failed" ? ShieldX : running.of === "drone" ? Bot : running.of === "check" ? ShieldCheck : Scale, state: said, hue: "running" };
  }
  const waiting = view.waiting?.[0];
  return waiting === undefined ? undefined : { ...WAITING[waiting.kind], hue: "queued" };
}

/** A Session's last ten rows as bars, a message tall and a tool call short, with a faint floor where it has said less. */
function sparkOf(session: Session): number[] {
  const rows = session.rows.slice(-10).map((row) => (row.kind === "message" ? 1 : row.kind === "tool" ? 0.45 : 0.25));
  return [...Array.from({ length: 10 - rows.length }, () => 0.12), ...rows];
}

export const age = (at: string | undefined, now: number): string => {
  if (at === undefined) return "";
  const minutes = Math.max(0, Math.round((now - Date.parse(at)) / 60_000));
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 60 * 24) return `${Math.round(minutes / 60)}h`;
  return `${Math.round(minutes / 60 / 24)}d`;
};

const worded = (status: string) => status.replaceAll("_", " ");

const noop = () => {};

export type Hosts = {
  onOpen: (jobId: string) => void;
  onOpenSession: (sessionId: string) => void;
  onOpenLink: (address: string) => void;
  /** Hands main's red to a Job, as the merge line's band does. */
  onFix?: ((fix: FixMain) => void) | undefined;
  /** Puts a request on the approval gate, as the composer does. */
  onPropose?: ((request: string) => void) | undefined;
  /** A Job's row acts, each asking as the Board row's does. */
  onKill?: ((jobId: string) => void) | undefined;
  onRedispatch?: ((jobId: string) => void) | undefined;
  onClear?: ((jobId: string) => void) | undefined;
  onPausing?: ((act: PauseAct, jobId: string) => void) | undefined;
  /** Says a sentence as a toast: a refusal the panel has to name. */
  onTell?: ((sentence: string) => void) | undefined;
  /** Where the cursor is, for Helm's footer: the Job picked, or null. */
  onCursor?: ((jobId: string | null) => void) | undefined;
};

/**
 * A failing pull request handed to a Drone: it reads the failure, says what it would change, and
 * pushes to the pull request only once the owner says so. On a Fleet, Send proposes that Job and its
 * findings come back as its own call; in the mock, where no Job runs, the look plays out here on a timer.
 */
function PullFix({ number, onOpen, onDone, onSend }: { number: number; onOpen: () => void; onDone: () => void; onSend?: () => void }) {
  const [stage, setStage] = useState<"idle" | "looking" | "found">("idle");
  useEffect(() => {
    if (stage !== "looking") return;
    const timer = window.setTimeout(() => setStage("found"), 2400);
    return () => window.clearTimeout(timer);
  }, [stage]);
  if (stage === "idle")
    return (
      <>
        <Button variant="primary" onClick={() => (onSend === undefined ? setStage("looking") : (onSend(), onDone()))}>Send a Drone</Button>
        <Button variant="secondary" onClick={onOpen}>Open pull request</Button>
      </>
    );
  if (stage === "looking")
    return (
      <div className="armada-pullfix" data-stage="looking">
        <LoaderCircle size={16} aria-hidden="true" className="armada-pullfix__spin" />
        <span>Drone reading the failed checks on #{number}</span>
      </div>
    );
  return (
    <div className="armada-pullfix" data-stage="found">
      <h3 className="armada-call__label">What the Drone would change</h3>
      <ul className="armada-pullfix__list">
        <li>PauseMarker is written before the Job row exists, so the foreign key fails on a fresh store</li>
        <li>Move the insert after create_job in store/jobs.rs, one call site</li>
        <li>Add the missing migration test the check names</li>
      </ul>
      <div className="armada-call__acts">
        <Button variant="primary" onClick={onDone}>Push to #{number}</Button>
        <Button variant="secondary" onClick={() => setStage("idle")}>Not now</Button>
      </div>
    </div>
  );
}

/**
 * A Drone gone quiet: tell it something and it picks up from there, start the step again on a
 * fresh Drone, or open the Job. Mock only: each press clears the call.
 */
function StuckDrone({ onDone, onOpen }: { onDone: () => void; onOpen: () => void }) {
  const [words, setWords] = useState("");
  return (
    <div className="armada-stuck">
      <Textarea label="Tell the Drone" rows={2} value={words} onChange={(event) => setWords(event.target.value)} />
      <div className="armada-call__acts">
        <Button variant="primary" disabled={words.trim() === ""} onClick={onDone}>Send</Button>
        <Button variant="secondary" onClick={onDone}>Restart the step</Button>
        <Button variant="secondary" onClick={onDone}>Fresh Drone</Button>
        <Button variant="ghost" onClick={onOpen}>Open Job</Button>
      </div>
    </div>
  );
}

/**
 * The Board's keys on the Dashboard list in front, read off the window as the Board's lists read
 * them (`boardPressOf`): j and k move, Enter, o or the row's verb key open what is picked, x asks to
 * kill a Job that can still be killed. n is Overview's own. `at` is the picked row's place.
 */
export function useBoardKeys(
  rows: readonly { key: string; job?: JobSummary | undefined }[],
  at: number,
  pick: (key: string) => void,
  hosts: Hosts,
  listening = true,
): void {
  useListKeydown((event) => {
    if (!listening) return;
    const read = boardPressOf(event);
    if (read === null) return;
    const here = rows[at];
    switch (read.act) {
      case "move": {
        const next = rows[Math.min(rows.length - 1, Math.max(0, at + read.by))];
        if (next === undefined) return;
        pick(next.key);
        break;
      }
      case "open":
      case "verb":
        if (here?.job !== undefined) hosts.onOpen(here.job.id);
        else if (sessionIdOf(here?.key ?? "") !== undefined) hosts.onOpenSession(sessionIdOf(here!.key)!);
        else return;
        break;
      case "kill":
        if (here?.job === undefined || isTerminal(here.job) || hosts.onKill === undefined) return;
        hosts.onKill(here.job.id);
        break;
      default:
        return;
    }
    event.preventDefault();
  });
}

/** Tells Helm's footer which Job the cursor is on, and that it is on none once the list goes. */
export function useCursor(job: JobSummary | undefined, onCursor: Hosts["onCursor"]): void {
  useEffect(() => onCursor?.(job?.id ?? null), [job?.id]);
  useEffect(() => () => onCursor?.(null), []);
}

/** A Plan decision, answered in place: one question at a time, the answer sent at the last. */
function PlanAnswer({ decisions, done }: { decisions: Extract<CallAskView, { kind: "plan" }>["decisions"]; done: () => void }) {
  const [at, setAt] = useState(0);
  const [picked, setPicked] = useState<string>();
  const decision = decisions[at]!;
  return (
    <div className="armada-dashboard__answer">
      <RadioGroup label={decision.question}>
        {decision.options.map((option) => (
          <Radio key={option.id} name={decision.id} checked={picked === option.id} onChange={() => setPicked(option.id)}>
            {option.label}
          </Radio>
        ))}
      </RadioGroup>
      <Button
        variant="primary"
        disabled={picked === undefined}
        onClick={() => (at + 1 < decisions.length ? (setAt(at + 1), setPicked(undefined)) : done())}
      >
        Answer
      </Button>
    </div>
  );
}

function jobItems(job: JobSummary, view: CallView | undefined, hosts: Hosts): Item[] {
  const open = () => hosts.onOpen(job.id);
  const where = job.handle;
  const items: Item[] = [];
  for (const ask of view?.asks ?? []) {
    if (ask.kind === "plan") {
      items.push({
        key: `${job.id}:${ask.key}`,
        owner: job.id,
        job,
        ...(view?.request === undefined ? {} : { request: view.request }),
        ...(view?.about === undefined ? {} : { about: view.about }),
        ...(ask.context === undefined ? {} : { context: ask.context, contextHead: "Why it asks" }),
        decisions: ask.decisions,
        icon: MessageSquare,
        kind: "Plan question",
        title: titleOf(job),
        fact: ask.decisions[0]?.question ?? "",
        at: job.started_at,
        hue: "ask",
        where,
        body: [["Step", "Plan"]],
        acts: (done) => <PlanAnswer decisions={ask.decisions} done={done} />,
      });
    } else {
      items.push({
        key: `${job.id}:${ask.key}`,
        owner: job.id,
        job,
        ...(view?.request === undefined ? {} : { request: view.request }),
        ...(view?.about === undefined ? {} : { about: view.about }),
        ...(ask.context === undefined ? {} : { context: ask.context, contextHead: ask.kind === "judge" ? "The Judge's words" : "The Drone's words" }),
        icon: ask.kind === "judge" ? Scale : Bot,
        kind: ask.kind === "judge" ? "Judge question" : "Drone question",
        title: titleOf(job),
        fact: ask.text,
        at: job.started_at,
        hue: "ask",
        where,
        body: [["From", ask.name]],
        said: [ask.text],
        acts: () => <Button variant="primary" onClick={open}>Open Job</Button>,
      });
    }
  }
  for (const issue of view?.issues ?? []) {
    items.push({
      key: `${job.id}:${issue.key}`,
      owner: job.id,
      job,
      ...(view?.request === undefined ? {} : { request: view.request }),
        ...(view?.about === undefined ? {} : { about: view.about }),
      ...(issue.context === undefined ? {} : { context: issue.context, contextHead: issue.of === "check" ? "Check output" : "The Drone's last steps" }),
      icon: issue.of === "check" ? ShieldX : Bot,
      kind: issue.said,
      title: titleOf(job),
      fact: issue.text,
      at: job.started_at,
      hue: "issue",
      where,
      body: [["On", issue.name]],
      said: [issue.text],
      acts: (done) =>
        issue.of === "drone" ? (
          <StuckDrone onDone={done} onOpen={open} />
        ) : (
          <>
            <Button variant="primary" onClick={done}>Retry</Button>
            <Button variant="secondary" onClick={open}>Open Job</Button>
          </>
        ),
    });
  }
  return items;
}

function sessionItem(session: Session, hosts: Hosts): Item {
  const last = [...session.rows].reverse().find((row) => row.kind === "message");
  const open = () => hosts.onOpenSession(session.id);
  const ended = session.dead === "ended";
  const working = session.turn.state === "working";
  const waiting = session.asked !== undefined && !ended && !working;
  return {
    key: `session:${session.id}`,
    owner: `session:${session.id}`,
    icon: SquareTerminal,
    kind: "Session",
    session: true,
    mark: ended ? Unplug : waiting ? Eye : working ? CircleDot : Check,
    state: waiting ? "Waiting on a command" : ended ? "Ended" : working ? "Working" : "Idle",
    spark: sparkOf(session),
    title: session.title ?? session.address ?? session.id,
    fact: waiting ? session.asked!.command : ended ? "ended" : working ? "working" : "idle",
    at: session.lastTurnAt,
    hue: waiting ? "ask" : ended ? "ok" : working ? "running" : "queued",
    live: working,
    where: session.address ?? "",
    body: waiting ? [["Asks to run", session.asked!.command]] : [],
    ...(last !== undefined && last.kind === "message" ? { said: [last.text] } : {}),
    doing: waiting ? "Waiting on a command" : ended ? "Ended" : working ? "Working" : "Idle",
    context: session.rows.flatMap((row) => (row.kind === "message" ? [row.text] : [])).slice(-3),
    contextHead: "Last messages",
    acts: (done) =>
      waiting ? (
        <>
          <Button variant="primary" onClick={done}>Allow</Button>
          <Button variant="secondary" onClick={done}>Deny</Button>
          <Button variant="ghost" onClick={open}>Open Session</Button>
        </>
      ) : (
        <Button variant="secondary" onClick={open}>Open Session</Button>
      ),
  };
}

/** One thing a Session waits on, as a call: one card per item, in the order they began. */
function waitingItem({ session, item }: WaitingCall, hosts: Hosts): Item {
  const last = [...session.rows].reverse().find((row) => row.kind === "message");
  return {
    key: `session:${session.id}:${item.id}`,
    owner: `session:${session.id}`,
    icon: SquareTerminal,
    kind: item.source === "ask_card" ? "Session question" : item.source === "walk" ? "Session walk" : "Session",
    session: true,
    mark: Eye,
    state: item.source === "permission" ? "Waiting on a command" : "Waiting on you",
    spark: sparkOf(session),
    title: session.title ?? session.address ?? session.id,
    fact: item.text,
    at: item.since,
    hue: "ask",
    where: session.address ?? "",
    body: [],
    ...(last !== undefined && last.kind === "message" ? { said: [last.text] } : {}),
    doing: "Waiting on you",
    context: session.rows.flatMap((row) => (row.kind === "message" ? [row.text] : [])).slice(-3),
    contextHead: "Last messages",
    waiting: { sessionId: session.id, item },
    acts: () => <Button variant="secondary" onClick={() => hosts.onOpenSession(session.id)}>Open Session</Button>,
  };
}

/** `rehearsed` is the mock, where a Drone's look at a pull request plays out in place on a timer. */
function lineItems(state: BridgeState, tab: DashboardTab, hosts: Hosts, rehearsed: boolean): Item[] {
  const items: Item[] = [];
  for (const view of viewsOf(state)) {
    const where = view.name ?? "Merge line";
    const main = view.hub?.main;
    if (tab === "command-central") {
      if (main?.state === "red" && main.taken === undefined && (main.checking ?? []).length === 0) {
        items.push({
          key: `main:${view.root}`,
          owner: "line",
          icon: GitMerge,
          kind: "Main",
          title: "main is red",
          fact: [main.red.check, main.red.test].filter(Boolean).join(" · "),
          hue: "issue",
          where,
          body: [["Check", main.red.check], ...(main.red.test === undefined ? [] : [["Test", main.red.test] as const])],
          doing: "Red, nobody on it",
          context: [
            `check   ${main.red.check}`,
            ...(main.red.test === undefined ? [] : [`test    ${main.red.test}`]),
            ...(main.red.merge === undefined ? [] : [`broke   #${main.red.merge.number} ${main.red.merge.branch}`]),
          ],
          contextHead: "What broke",
          acts: (done) => (
            <Button variant="primary" disabled={hosts.onFix === undefined} onClick={() => (hosts.onFix?.({ root: view.root }), done())}>
              Hand to a Job
            </Button>
          ),
        });
      }
      for (const pull of view.hub?.pulls ?? []) {
        if (pull.ci !== "failed" && pull.queue?.state !== "unmergeable") continue;
        items.push({
          key: `pull:${pull.number}`,
          icon: GitPullRequest,
          kind: "Pull request",
          title: `#${pull.number} ${pull.branch}`,
          fact: pull.ci === "failed" ? "checks failed" : "unmergeable",
          hue: "issue",
          where,
          body: [["Branch", pull.branch]],
          acts: (done) => (
            <PullFix
              number={pull.number}
              onOpen={() => hosts.onOpenLink(pull.url)}
              onDone={done}
              {...(rehearsed || hosts.onPropose === undefined
                ? {}
                : {
                    onSend: () =>
                      hosts.onPropose!(
                        `The checks on pull request #${pull.number} (${pull.url}, branch ${pull.branch}) failed. Read the failure, say what you would change, and push to the pull request only once I agree.`,
                      ),
                  })}
            />
          ),
        });
      }
    }
    // Done is Jobs and Sessions; what landed is the merge line's, not an item.
    for (const entry of tab === "running" ? view.line : []) {
      items.push({
        key: `line:${view.root}:${entry.branch}`,
        icon: GitMerge,
        kind: "In the merge line",
        title: entry.job?.title ?? entry.branch,
        fact: `${entry.place === undefined ? "" : `#${entry.place} · `}${worded(entry.state)}`,
        hue: "running",
        live: entry.state === "gating" || entry.state === "merging" || entry.state === "preparing",
        where,
        body: [["Branch", entry.branch], ...(entry.pr === undefined ? [] : [["Pull request", `#${entry.pr.number}`] as const])],
        acts: () =>
          entry.job === undefined ? null : <Button variant="secondary" onClick={() => hosts.onOpen(entry.job!.id)}>Open Job</Button>,
      });
    }
  }
  return items;
}

/** Whether the owner is wanted anywhere: the quick box shows only where this is false. */
export function useNeedsYou(state: BridgeState, picked: RepositorySummary | null, nowViews: Readonly<Record<string, CallView>> | undefined): boolean {
  const items = useItems("command-central", state, picked, nowViews, { onOpen: () => {}, onOpenSession: () => {}, onOpenLink: () => {} }, new Set());
  return items.length > 0;
}

export function useItems(
  tab: DashboardTab,
  state: BridgeState,
  picked: RepositorySummary | null,
  nowViews: Readonly<Record<string, CallView>> | undefined,
  hosts: Hosts,
  answered: ReadonlySet<string>,
): Item[] {
  const sessions = useSessions();
  const nows = useContext(Nows);
  const dismissed = useDismissed();
  const calls = tab === "command-central";
  const items = useMemo(() => {
    const read = overviewListsOf(state.jobs, picked);
    const of = (ids: readonly string[]) => read.sections.filter((one) => ids.includes(one.id)).flatMap((one) => one.jobs);
    const items: Item[] = [];
    if (tab === "command-central") {
      for (const job of of(["needs-you", "running", "queued", "other"])) {
        const asked = jobItems(job, nowViews?.[job.id], hosts);
        if (asked.length > 0) items.push(...asked);
        else if (read.sections.find((one) => one.id === "needs-you")?.jobs.includes(job))
          items.push({
            key: job.id,
            owner: job.id,
            icon: Eye,
            kind: "Job",
            title: titleOf(job),
            fact: worded(job.status),
            at: job.ended_at ?? job.started_at,
            hue: "ask",
            where: job.handle,
            job,
            ...(nowViews?.[job.id]?.request === undefined ? {} : { request: nowViews[job.id]!.request! }),
            ...(nowViews?.[job.id]?.about === undefined ? {} : { about: nowViews[job.id]!.about! }),
            body: [["Status", worded(job.status)], ...(job.branch === undefined ? [] : [["Branch", job.branch] as const])],
            acts: () => <JobActs job={job} stale={false} onOpen={hosts.onOpen} onKill={hosts.onKill ?? noop} onRedispatch={hosts.onRedispatch ?? noop} onClear={hosts.onClear ?? noop} {...(hosts.onPausing === undefined ? {} : { onPausing: hosts.onPausing })} />,
          });
      }
      items.push(...callsFromWaiting(sessions).map((one) => waitingItem(one, hosts)));
    } else {
      const jobs = tab === "running" ? of(["running", "queued", "other"]) : of(["recently-ended", "done"]);
      for (const job of jobs) {
        const queued = read.sections.find((one) => one.id === "queued")?.jobs.includes(job) ?? false;
        const failed = /fail|killed|rejected/.test(job.status);
        const live = nowViews?.[job.id]?.running ?? [];
        items.push({
          key: job.id,
          icon: tab === "done" ? (failed ? CircleX : CircleCheck) : queued ? Clock : Workflow,
          kind: tab === "done" ? (failed ? "Ended badly" : "Done") : queued ? "Queued" : "Running",
          title: titleOf(job),
          fact: live[0]?.line ?? job.current_step_id ?? worded(job.status),
          at: tab === "done" ? job.ended_at : job.started_at ?? job.created_at,
          hue: tab === "done" ? (failed ? "bad" : "ok") : queued ? "queued" : "running",
          live: tab === "running" && !queued,
          where: job.handle,
          job,
          ...(nowViews?.[job.id]?.request === undefined ? {} : { request: nowViews[job.id]!.request! }),
            ...(nowViews?.[job.id]?.about === undefined ? {} : { about: nowViews[job.id]!.about! }),
          ...(live.flatMap((one) => one.tail ?? []).length === 0 ? {} : { context: live.flatMap((one) => one.tail ?? []), contextHead: "Output" }),
          body: [
            ["Status", worded(job.status)],
            ...(job.current_step_id === undefined ? [] : [["Step", job.current_step_id] as const]),
            ...(job.branch === undefined ? [] : [["Branch", job.branch] as const]),
          ],
          said: live.flatMap((one) => one.tail ?? (one.line === undefined ? [] : [one.line])),
          acts: () => <JobActs job={job} stale={false} onOpen={hosts.onOpen} onKill={hosts.onKill ?? noop} onRedispatch={hosts.onRedispatch ?? noop} onClear={hosts.onClear ?? noop} {...(hosts.onPausing === undefined ? {} : { onPausing: hosts.onPausing })} />,
        });
      }
      items.push(
        ...sessions
          .filter((one) => (tab === "done" ? one.dead === "ended" : one.dead === undefined && one.asked === undefined))
          .map((one) => sessionItem(one, hosts)),
      );
    }
    items.push(...lineItems(state, tab, hosts, nowViews !== undefined));
    // A Job's pips come from its workflow and its icon from the first thing active on it.
    const drawn = items.map((item): Item => {
      if (item.job === undefined || tab === "command-central") return item;
      const job = item.job;
      const steps = (state.holds.workflows.find((one) => one.id === job.workflow_id)?.steps ?? []).map((step) => ({ id: step.step_id, label: step.label }));
      const at = tab === "done" && item.hue !== "bad" ? steps.length : Math.max(0, steps.findIndex((step) => step.id === job.current_step_id));
      const active = tab === "done" ? undefined : activeOf(nows?.[job.id]);
      return { ...item, ...(steps.length === 0 ? {} : { steps, stepAt: at }), ...(active === undefined ? {} : { ...active, live: true }) };
    });
    return drawn.filter((item) => !answered.has(item.key));
  }, [tab, state, picked, nowViews, nows, sessions, answered, dismissed]);
  // A call dismissed for good stays gone while its state stands; once the state clears, the next one shows.
  const standing = useMemo(() => new Set(items.map(identityOf)), [items]);
  const board = state.jobs.length > 0 || viewsOf(state).length > 0;
  useEffect(() => {
    if (calls) forgetCleared(standing, board);
  }, [calls, standing, board]);
  return useMemo(() => (calls ? items.filter((item) => !dismissed.has(identityOf(item))) : items), [calls, items, dismissed]);
}
