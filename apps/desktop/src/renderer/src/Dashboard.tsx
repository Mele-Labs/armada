// What the Dashboard's panel reads: every Job, Session and merge-line item as one `Item`, cut three
// ways — what needs the owner, what is under way, what is over — with the keys the Board's lists
// answer. `cockpit/` draws them. Mock only: what a Job asks comes from the draft's Now views, which
// Fleet does not serve.

import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  BatteryLow,
  Bot,
  Box,
  Check,
  CircleCheck,
  CircleDot,
  CircleX,
  Clock,
  Cpu,
  Eye,
  FolderGit2,
  GitMerge,
  GitPullRequest,
  Compass,
  LoaderCircle,
  Megaphone,
  MessageSquare,
  Moon,
  OctagonAlert,
  Power,
  Reply,
  Scale,
  ShieldCheck,
  ShieldX,
  SquareTerminal,
  Unplug,
  Waypoints,
  Workflow,
  type LucideIcon,
} from "lucide-react";
import { Button, Radio, RadioGroup, limitHeldSaid, type FixRefusal } from "@armada/components";
import type { BayActs } from "@armada/cleanup";
import type { FixMain, JobSummary, Outcome, RepositorySummary, WaitingItem } from "@armada/protocol";
import type { AboutFiles, AboutLink, CallAskView, CallIssueView, CallView } from "@armada/jobs/draft/calls";
import type { AskedOf, NowView } from "@armada/jobs/draft/now";
import type { NowActed } from "@armada/jobs";
import type { Session } from "@armada/screens/src/draft/sessions";
import { overviewListsOf, type DashboardTab } from "@armada/overview";
import { JobActs, isTerminal, titleOf, type PauseAct } from "@armada/screens";
import { boardPressOf } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { viewsOf } from "./merge-line";
import { stateOf } from "./session-state";
import { useSessions } from "./sessions-draft";
import { lineOf } from "./session-activity";
import { useSleepDecided, type MorningDecided } from "./sleep";
import { forgetCleared, identityOf, useDismissed } from "./cockpit/dismissed";
import { useOpenWalks } from "./cockpit/open-walks";
import { failedAt, isOnIt, ownerOfCall, type Owner } from "./cockpit/owner";
import type { JobAnswered } from "./cockpit/answer-job";
import { checkoutCall } from "./cockpit/checkout";
import { MERGE_BY_HAND } from "./cockpit/horizon";
import { ByHandActs } from "./merge-by-hand";
import { callsFromWaiting, sessionIdOf, type WaitingCall } from "./cockpit/waiting";
import { useViewedVersion, viewedAt } from "./cockpit/viewed";

/** The frame's hue: what the row wants of the owner, or how it stands. */
type Hue = "ask" | "issue" | "running" | "queued" | "ok" | "bad";

export type Item = {
  key: string;
  /** The lane it lights on the fleet board: a Job's id, `session:<id>`, or `line`. */
  owner?: string;
  /** The Plan decisions, where the item is one: the call answers them as tiles. */
  decisions?: Extract<CallAskView, { kind: "plan" }>["decisions"];
  /** The pick on a Commander's card, sent to Fleet, which wakes the Commander on it or refuses in these words. */
  answerCard?: (optionId: string) => Promise<JobAnswered>;
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
  /** The Job is being proposed and its workflow has not landed: the tile holds the steps' place with the settling mark. */
  settling?: true;
  /** What a running Session is doing, for its tile: the todo being done, else its detected action. */
  line?: string;
  /** A Session's recent cadence, each bar 0 to 1: a message tall, a tool call short. */
  spark?: readonly number[];
  /** What went wrong on the Job, where the call is that: the card shows it and offers the fixes. */
  issue?: CallIssueView;
  /** The decision Sleep made, where the item is one: the call keeps it or changes it. */
  sleep?: MorningDecided;
  /** What a Session's call waits on: the item, which is what the call answers. */
  waiting?: { sessionId: string; item: WaitingItem };
  /** What a Job's ask is answered through, where Fleet served it: the question or Judge call, or a command. */
  asked?: AskedOf;
  /** Drawn from what Fleet serves, so its acts are real ones: Open Job, and no mock stub. */
  served?: true;
  /** Who holds the pull request or red main the call is about, where somebody does: the card draws what they are doing. */
  ownedBy?: Owner;
};

/** What the Dashboard knows of each Job's Now, by Job id: the draft's, else what Fleet serves for the active Jobs. */
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
  onFix?: ((fix: FixMain) => void | Promise<FixRefusal | undefined | void>) | undefined;
  /** Sends main's red to a Session as a message, as the merge line's band does. */
  onSendToSession?: ((sessionId: string, message: string) => void) | undefined;
  /** Puts a request on the approval gate, as the composer does. */
  onPropose?: ((request: string) => void) | undefined;
  /** A Job's row acts, each asking as the Board row's does. */
  onKill?: ((jobId: string) => void) | undefined;
  onRedispatch?: ((jobId: string) => void) | undefined;
  onClear?: ((jobId: string) => void) | undefined;
  onPausing?: ((act: PauseAct, jobId: string) => void) | undefined;
  /** Says a sentence as a toast: a refusal the panel has to name. */
  onTell?: ((sentence: string) => void) | undefined;
  /** A quick act pressed on a Job's Now panel. Absent, the panel says what the act would do. */
  onNowAct?: ((jobId: string, acted: NowActed) => void) | undefined;
  /** A pick on a Job's Commander card, by the card's id and the option's. `commander-acts.ts`. */
  onAnswerCard?: ((jobId: string, card: string, option: string) => Promise<JobAnswered>) | undefined;
  /** Releases a slot an agent session holds, for the holder the confirm showed: a terminal's process, or a hosted Session's id. */
  onReleaseSlot?: ((manifestId: string, slot: number, holder: string) => Promise<Outcome>) | undefined;
  /** Every other act the worktree pool's tile panel offers, which a bay's pane draws too. Absent, the pane offers none of them. */
  bayActs?: Omit<BayActs, "onRescue"> | undefined;
  /** Asks Fleet for the worktree pool while the bays are drawn, and lets go after. */
  onWantHeld?: ((want: boolean) => void) | undefined;
  /** Where the cursor is, for Helm's footer: the Job picked, or null. */
  onCursor?: ((jobId: string | null) => void) | undefined;
  /** Goes to a page by its surface id: the deck's links to the pages no tab carries (Worktree Slots, the merge line, the Manifest). */
  onGoTo?: ((surface: string) => void) | undefined;
  /** Picks a repository, as the title row's picker does; null is All repositories. */
  onPickRepository?: ((root: string | null) => void) | undefined;
  /** Opens the Add a repository dialog, the act the title row's picker carries as its last row. */
  onAddRepository?: (() => void) | undefined;
  /** Rescues a stranded slot: a Scout started or stopped, and the work picked up, stashed or scrapped. Absent, the deck draws no rescue acts. */
  onRescueSlot?: BridgeApi["rescueSlot"] | undefined;
  /** Merges a Job's pull request, the command the review gate sends. Absent, a Job's Merge is not offered. */
  onMergeJob?: ((jobId: string) => void | Promise<void>) | undefined;
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
function PlanAnswer({
  decisions,
  done,
  told,
  refused,
}: {
  decisions: Extract<CallAskView, { kind: "plan" }>["decisions"];
  done: () => void;
  /** Sends the pick to Fleet, where somebody besides the card hears it: the card clears once it is taken. */
  told?: ((option: string) => Promise<JobAnswered>) | undefined;
  refused?: ((said: string) => void) | undefined;
}) {
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
        onClick={() => (at + 1 < decisions.length ? (setAt(at + 1), setPicked(undefined)) : told === undefined ? done() : void told(picked!).then((one) => (one.kind === "answered" ? done() : refused?.(one.said))))}
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
    if (ask.kind === "plan" || ask.kind === "commander") {
      const commander = ask.kind === "commander";
      const answerCard = ask.kind === "commander" && hosts.onAnswerCard !== undefined ? (option: string) => hosts.onAnswerCard!(job.id, ask.card, option) : undefined;
      items.push({
        key: `${job.id}:${ask.key}`,
        owner: job.id,
        job,
        ...(view?.request === undefined ? {} : { request: view.request }),
        ...(view?.about === undefined ? {} : { about: view.about }),
        ...(ask.context === undefined ? {} : { context: ask.context, contextHead: "Why it asks" }),
        decisions: ask.decisions,
        ...(answerCard === undefined ? {} : { answerCard }),
        icon: commander ? Compass : MessageSquare,
        kind: commander ? "Commander asks" : "Plan question",
        title: titleOf(job),
        fact: ask.decisions[0]?.question ?? "",
        at: job.started_at,
        hue: "ask",
        where,
        body: commander ? [["From", "Commander"]] : [["Step", "Plan"]],
        acts: (done) => <PlanAnswer decisions={ask.decisions} done={done} told={answerCard} refused={hosts.onTell} />,
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
        ...(ask.asked === undefined ? {} : { asked: ask.asked }),
        acts: () => <Button variant="primary" onClick={open}>Open Job</Button>,
      });
    }
  }
  for (const issue of view?.issues ?? []) {
    items.push({
      key: `${job.id}:${issue.key}`,
      owner: job.id,
      job,
      issue,
      ...(view?.request === undefined ? {} : { request: view.request }),
      ...(view?.about === undefined ? {} : { about: view.about }),
      ...(issue.context === undefined ? {} : { context: issue.context, contextHead: issue.of === "check" ? "Check output" : "The Drone's last steps" }),
      icon: issue.of === "check" ? ShieldX : issue.of === "judge" ? Scale : issue.of === "clash" ? GitMerge : Bot,
      kind: issue.said,
      title: titleOf(job),
      fact: issue.text,
      at: job.started_at,
      hue: "issue",
      where,
      body: [["On", issue.name]],
      said: [issue.text],
      ...(view?.served === true ? { served: true as const } : {}),
      acts: () => null,
    });
  }
  return items;
}

function sessionItem(session: Session, hosts: Hosts): Item {
  const last = [...session.rows].reverse().find((row) => row.kind === "message");
  const open = () => hosts.onOpenSession(session.id);
  // The state is the one the Sessions list reads (`session-state.ts`), so a tile never disagrees with its row.
  const read = stateOf(session).state;
  const ended = read === "ended";
  const stuck = read === "stuck";
  const replied = read === "replied";
  const finished = read === "done";
  const working = read === "working" || stuck;
  const waiting = session.asked !== undefined && !ended;
  const limited = ended ? undefined : session.limitedUntil;
  const line = lineOf(session);
  const said = stuck ? "Stuck" : replied ? "Unread reply" : finished ? "Done" : working ? "Working" : "Idle";
  return {
    key: `session:${session.id}`,
    owner: `session:${session.id}`,
    icon: SquareTerminal,
    kind: "Session",
    session: true,
    mark: ended ? Unplug : limited !== undefined ? BatteryLow : waiting ? Eye : stuck ? Clock : working ? CircleDot : replied ? Reply : finished ? Power : Check,
    state: limited !== undefined ? limitHeldSaid(limited) : waiting ? "Waiting on a command" : ended ? "Ended" : said,
    spark: sparkOf(session),
    ...(line === undefined ? {} : { line }),
    title: session.title ?? session.address ?? session.id,
    fact: limited !== undefined ? "held" : waiting ? session.asked!.command : ended ? "ended" : said.toLowerCase(),
    at: session.lastTurnAt,
    hue: waiting || finished ? "ask" : ended ? "ok" : stuck ? "issue" : working && limited === undefined ? "running" : "queued",
    live: read === "working" && limited === undefined,
    where: session.address ?? "",
    body: waiting ? [["Asks to run", session.asked!.command]] : [],
    ...(last !== undefined && last.kind === "message" ? { said: [last.text] } : {}),
    doing: waiting ? "Waiting on a command" : ended ? "Ended" : said,
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
    kind: item.source === "ask_card" || item.source === "window_ask" ? "Session question" : item.source === "walk" ? "Session walk" : "Session",
    session: true,
    mark: item.source === "done" ? Power : Eye,
    state: item.source === "permission" ? "Waiting on a command" : item.source === "done" ? "Done" : "Waiting on you",
    spark: sparkOf(session),
    title: session.title ?? session.address ?? session.id,
    fact: item.text,
    at: item.since,
    hue: "ask",
    where: session.address ?? "",
    body: [],
    ...(last !== undefined && last.kind === "message" ? { said: [last.text] } : {}),
    doing: item.source === "done" ? "Done" : "Waiting on you",
    context: session.rows.flatMap((row) => (row.kind === "message" ? [row.text] : [])).slice(-3),
    contextHead: "Last messages",
    waiting: { sessionId: session.id, item },
    acts: () => <Button variant="secondary" onClick={() => hosts.onOpenSession(session.id)}>Open Session</Button>,
  };
}

/** One thing Sleep decided for the owner, as a call: he agrees, changes it, or opens where it was decided. */
function sleepItem(one: MorningDecided): Item {
  return {
    key: `sleep:${one.id}`,
    icon: Moon,
    kind: "Decided while you slept",
    title: one.who,
    fact: one.asked,
    hue: "ask",
    where: "",
    body: [],
    context: [one.corrected ?? one.chose],
    contextHead: "Chose",
    sleep: one,
    acts: () => null,
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
      // A held checkout is told here and answered on the merge queue's checkout light, which opening the call pins.
      if (view.hub?.checkout !== undefined) {
        const held = checkoutCall(view.root, view.hub.checkout);
        items.push({
          key: `checkout:${view.root}`,
          owner: "line",
          icon: FolderGit2,
          kind: "Checkout",
          title: held.title,
          fact: held.fact,
          hue: "ask",
          where,
          body: held.facts.map((one) => [one.label, one.lines.map((line) => line.text).join("\n")] as const),
          acts: () => null,
        });
      }
      for (const pull of view.hub?.pulls ?? []) {
        const failing = pull.ci === "failed" || pull.queue?.state === "unmergeable";
        // A form edit the forge would not auto-merge waits for a person; opening the call opens its pull request.
        if (!failing && pull.mergeByHand === true) {
          items.push({
            key: `pull:${pull.number}`,
            owner: "line",
            icon: GitPullRequest,
            kind: "Pull request",
            title: `armada.yml edit #${pull.number}`,
            fact: MERGE_BY_HAND,
            hue: "ask",
            where,
            body: [["Branch", pull.branch]],
            acts: () => <ByHandActs number={pull.number} {...(pull.ci === undefined ? {} : { ci: pull.ci })} />,
          });
          continue;
        }
        if (!failing) continue;
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
  const slept = useSleepDecided();
  const walking = useOpenWalks();
  const viewed = useViewedVersion();
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
      // A walk whose window is open is being looked at, so it is not a call yet.
      const unseen = (one: WaitingCall) => !(one.item.source === "walk" && one.item.act !== undefined && walking.has(one.item.act.target));
      items.push(...callsFromWaiting(sessions, viewedAt).filter(unseen).map((one) => waitingItem(one, hosts)));
      items.push(...slept.map(sleepItem));
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
    // A failing pull request or red main whose owner is already on it is not a call; one that is, shows its owner.
    const views = viewsOf(state);
    const working = new Set((read.sections.find((one) => one.id === "running")?.jobs ?? []).map((job) => job.id));
    const told = new Map(state.jobs.flatMap((job) => (job.last_told_at === undefined ? [] : [[job.id, job.last_told_at] as const])));
    items.push(
      ...lineItems(state, tab, hosts, nowViews !== undefined && !Object.values(nowViews).some((view) => view.served === true)).flatMap((item): Item[] => {
        const owner = ownerOfCall(views, sessions, item.key);
        if (owner === undefined) return [item];
        return isOnIt(owner, sessions, working, told, failedAt(views, item.key)) ? [] : [{ ...item, ownedBy: owner }];
      }),
    );
    // A Job's pips come from its workflow and its icon from the first thing active on it.
    const drawn = items.map((item): Item => {
      if (item.job === undefined || tab === "command-central") return item;
      const job = item.job;
      const steps = (state.holds.workflows.find((one) => one.id === job.workflow_id)?.steps ?? []).map((step) => ({ id: step.step_id, label: step.label }));
      const at = tab === "done" && item.hue !== "bad" ? steps.length : Math.max(0, steps.findIndex((step) => step.id === job.current_step_id));
      const active = tab === "done" ? undefined : activeOf(nows?.[job.id]);
      // The workflow is the proposer's to settle: until it lands there is nothing to draw pips from.
      const settling = job.status === "proposing" && job.workflow_id === "";
      const limited = job.paused?.until;
      return { ...item, ...(steps.length === 0 ? {} : { steps, stepAt: at }), ...(settling ? { settling: true as const } : {}), ...(active === undefined ? {} : { ...active, live: true }), ...(limited === undefined ? {} : { mark: BatteryLow, state: limitHeldSaid(limited), live: false }) };
    });
    return drawn.filter((item) => !answered.has(item.key));
  }, [tab, state, picked, nowViews, nows, sessions, slept, walking, answered, dismissed, viewed]);
  // A call dismissed for good stays gone while its state stands; once the state clears, the next one shows.
  const standing = useMemo(() => new Set(items.map(identityOf)), [items]);
  const board = state.jobs.length > 0 || viewsOf(state).length > 0;
  useEffect(() => {
    if (calls) forgetCleared(standing, board);
  }, [calls, standing, board]);
  return useMemo(() => (calls ? items.filter((item) => !dismissed.has(identityOf(item))) : items), [calls, items, dismissed]);
}
