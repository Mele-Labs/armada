// What the Dashboard's panel reads: every Session and merge-line item as one `Item`, cut three
// ways — what needs the owner, what is under way, what is over — with the keys the lists answer.
// `cockpit/` draws them.

import { useEffect, useMemo, type ReactNode } from "react";
import {
  BatteryLow,
  Check,
  CircleDot,
  Clock,
  Eye,
  FolderGit2,
  GitMerge,
  GitPullRequest,
  Moon,
  Power,
  Reply,
  SquareTerminal,
  Unplug,
  type LucideIcon,
} from "lucide-react";
import { Button, briefOf, limitHeldSaid, sessionGlyphOf } from "@armada/components";
import type { BayActs } from "@armada/cleanup";
import type { Outcome, RepositorySummary, WaitingItem } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { DashboardTab } from "@armada/overview";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { viewsOf } from "./merge-line";
import { stateOf } from "./session-state";
import { useSessions } from "./sessions-draft";
import { lineOf } from "./session-activity";
import { useSleepDecided, type MorningDecided } from "./sleep";
import { useDispatching } from "./dispatch-panel/dispatching";
import { openInPanel } from "./shared-panel/store";
import { forgetCleared, identityOf, useDismissed } from "./cockpit/dismissed";
import { useOpenWalks } from "./cockpit/open-walks";
import { failedAt, isOnIt, ownerOfCall, workOwner, type Owner } from "./cockpit/owner";
import { checkoutCall } from "./cockpit/checkout";
import { MERGE_BY_HAND } from "./cockpit/horizon";
import { ByHandActs } from "./merge-by-hand";
import { callKindOf, callsFromWaiting, type WaitingCall } from "./cockpit/waiting";
import { useViewedVersion, viewedAt } from "./cockpit/viewed";
import type { LoadoutAt } from "./session-loadout";

/** The frame's hue: what the row wants of the owner, or how it stands. */
type Hue = "ask" | "issue" | "running" | "queued" | "ok" | "bad";

export type Item = {
  key: string;
  /** The lane it lights on the fleet board: `session:<id>`, or `line`. */
  owner?: string;
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
  /** What it is doing, for a thing with no steps. */
  doing?: string;
  /** The words leading up to it, or the output it failed with, and what to call them. */
  context?: readonly string[];
  contextHead?: string;
  acts: (done: () => void) => ReactNode;
  /** The tile's state icon where the kind's own would not say what is active: a Session's state. */
  mark?: LucideIcon;
  /** What that icon's tooltip says. The kind where absent. */
  state?: string;
  /** Whether the item is a Session. */
  session?: boolean;
  /** What a running Session is doing, for its tile: the todo being done, else its detected action. */
  line?: string;
  /** A Session's recent cadence, each bar 0 to 1: a message tall, a tool call short. */
  spark?: readonly number[];
  /** The decision Sleep made, where the item is one: the call keeps it or changes it. */
  sleep?: MorningDecided;
  /** What a Session's call waits on: the item, which is what the call answers. */
  waiting?: { sessionId: string; item: WaitingItem };
  /** Who holds the pull request or red main the call is about, where somebody does: the card draws what they are doing. */
  ownedBy?: Owner;
};

/** A Session's last ten rows as bars, a message tall and a tool call short, with a faint floor where it has said less. */
function sparkOf(session: Session): number[] {
  const rows = session.rows.slice(-10).map((row) => (row.kind === "message" ? 1 : row.kind === "tool" ? 0.45 : 0.25));
  return [...Array.from({ length: 10 - rows.length }, () => 0.12), ...rows];
}

const worded = (status: string) => status.replaceAll("_", " ");

export type Hosts = {
  onOpenSession: (sessionId: string) => void;
  onOpenLink: (address: string) => void;
  /** Opens a Session's Check run in its panel, where the page stands. Absent, a press opens the Session. */
  onOpenCheck?: ((sessionId: string, run: number) => void) | undefined;
  /** Opens a Session's Loadout in the same panel. Absent, the mark's press opens nothing. */
  onOpenLoadout?: ((sessionId: string, at?: LoadoutAt) => void) | undefined;
  /** Sends main's red to a Session as a message, as the merge line's band does. */
  onSendToSession?: ((sessionId: string, message: string) => void) | undefined;
  /** Says a sentence as a toast: a refusal the panel has to name. */
  onTell?: ((sentence: string) => void) | undefined;
  /** Releases a slot an agent session holds, for the holder the confirm showed: a terminal's process, or a hosted Session's id. */
  onReleaseSlot?: ((manifestId: string, slot: number, holder: string) => Promise<Outcome>) | undefined;
  /** Every other act the worktree pool's tile panel offers, which a bay's pane draws too. Absent, the pane offers none of them. */
  bayActs?: Omit<BayActs, "onRescue"> | undefined;
  /** Asks Fleet for the worktree pool while the bays are drawn, and lets go after. */
  onWantHeld?: ((want: boolean) => void) | undefined;
  /** Goes to a page by its surface id: the deck's links to the pages no tab carries (Worktree Slots, the merge line, the Manifest). */
  onGoTo?: ((surface: string) => void) | undefined;
  /** Picks a repository, as the title row's picker does; null is All repositories. */
  onPickRepository?: ((root: string | null) => void) | undefined;
  /** Opens the Add a repository dialog, the act the title row's picker carries as its last row. */
  onAddRepository?: (() => void) | undefined;
  /** Opens the same dialog on a new project, the row above it. */
  onNewProject?: (() => void) | undefined;
  /** Rescues a stranded slot: a Scout started or stopped, and the work picked up, stashed or scrapped. Absent, the deck draws no rescue acts. */
  onRescueSlot?: BridgeApi["rescueSlot"] | undefined;
};

/** Starts a Job on a brief for `root` and opens it in draft: the way a failing pull request or red main is handed on. */
function StartSession({ brief, root, onDone, children }: { brief: string; root: string; onDone: () => void; children?: ReactNode }) {
  const dispatching = useDispatching();
  return (
    <Button
      variant="primary"
      disabled={dispatching === undefined}
      onClick={() => (dispatching?.startDraft(brief, root), onDone())}
    >
      {children ?? "Start a Job"}
    </Button>
  );
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
  const drafted = read === "draft";
  const working = read === "working" || stuck;
  const waiting = session.asked !== undefined && !ended;
  const limited = ended ? undefined : session.limitedUntil;
  const line = lineOf(session);
  const said = drafted ? "Draft" : stuck ? "Stuck" : replied ? "Unread reply" : finished ? "Done" : working ? "Working" : "Idle";
  return {
    key: `session:${session.id}`,
    owner: `session:${session.id}`,
    icon: SquareTerminal,
    kind: "Job",
    session: true,
    mark: ended ? Unplug : limited !== undefined ? BatteryLow : waiting ? Eye : drafted ? sessionGlyphOf("draft") : stuck ? Clock : working ? CircleDot : replied ? Reply : finished ? Power : Check,
    state: limited !== undefined ? limitHeldSaid(limited) : waiting ? "Waiting on a command" : ended ? "Ended" : said,
    spark: sparkOf(session),
    ...(line === undefined ? {} : { line }),
    title: session.title ?? session.address ?? session.id,
    fact: limited !== undefined ? "held" : waiting ? session.asked!.command : ended ? "ended" : said.toLowerCase(),
    at: session.lastTurnAt,
    hue: waiting || finished || drafted ? "ask" : ended ? "ok" : stuck ? "issue" : working && limited === undefined ? "running" : "queued",
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
          <Button variant="ghost" onClick={open}>Open Job</Button>
        </>
      ) : (
        <Button variant="secondary" onClick={open}>Open Job</Button>
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
    kind: callKindOf(item.source),
    session: true,
    mark: item.source === "done" ? Power : stateOf(session).state === "unstarted" ? sessionGlyphOf("unstarted") : Eye,
    state: item.source === "permission" ? "Waiting on a command" : item.source === "done" ? "Done" : stateOf(session).state === "unstarted" ? "Failed to start" : "Waiting on you",
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
    acts: () => <Button variant="secondary" onClick={() => hosts.onOpenSession(session.id)}>Open Job</Button>,
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

/** `sessions` finds who made a branch in the line, so its row opens that Session. */
function lineItems(state: BridgeState, tab: DashboardTab, hosts: Hosts, sessions: readonly Session[]): Item[] {
  const items: Item[] = [];
  for (const view of viewsOf(state)) {
    const where = view.name ?? "Merge line";
    const main = view.hub?.main;
    if (tab === "command-central") {
      if (main?.state === "red" && (main.checking ?? []).length === 0) {
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
          acts: (done) => <StartSession brief={briefOf(main.red)} root={view.root} onDone={done} />,
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
            acts: () => <ByHandActs root={view.root} number={pull.number} {...(pull.ci === undefined ? {} : { ci: pull.ci })} />,
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
            <>
              <StartSession
                brief={`The checks on pull request #${pull.number} (${pull.url}, branch ${pull.branch}) failed. Read the failure, say what you would change, and push to the pull request only once I agree.`}
                root={view.root}
                onDone={done}
              />
              <Button variant="secondary" onClick={() => openInPanel({ kind: "pr", number: pull.number, address: pull.url, root: view.root, branch: pull.branch })}>Open pull request</Button>
            </>
          ),
        });
      }
    }
    // Done is Sessions; what landed is the merge line's, not an item.
    for (const entry of tab === "running" ? view.line : []) {
      const owner = workOwner(sessions, entry.pr?.number ?? -1, entry.branch);
      items.push({
        key: `line:${view.root}:${entry.branch}`,
        icon: GitMerge,
        kind: "In the merge line",
        title: entry.branch,
        fact: `${entry.place === undefined ? "" : `#${entry.place} · `}${worded(entry.state)}`,
        hue: "running",
        live: entry.state === "gating" || entry.state === "merging" || entry.state === "preparing",
        where,
        body: [["Branch", entry.branch], ...(entry.pr === undefined ? [] : [["Pull request", `#${entry.pr.number}`] as const])],
        acts: () =>
          owner === undefined ? null : <Button variant="secondary" onClick={() => hosts.onOpenSession(owner.id)}>Open Job</Button>,
      });
    }
  }
  return items;
}

export function useItems(
  tab: DashboardTab,
  state: BridgeState,
  picked: RepositorySummary | null,
  hosts: Hosts,
  answered: ReadonlySet<string>,
): Item[] {
  const sessions = useSessions();
  const dismissed = useDismissed();
  const slept = useSleepDecided();
  const walking = useOpenWalks();
  const viewed = useViewedVersion();
  const calls = tab === "command-central";
  const items = useMemo(() => {
    const items: Item[] = [];
    if (tab === "command-central") {
      // A walk whose window is open is being looked at, so it is not a call yet.
      const unseen = (one: WaitingCall) => !(one.item.source === "walk" && one.item.act !== undefined && walking.has(one.item.act.target));
      items.push(...callsFromWaiting(sessions, viewedAt).filter(unseen).map((one) => waitingItem(one, hosts)));
      items.push(...slept.map(sleepItem));
    } else {
      items.push(
        ...sessions
          .filter((one) => (tab === "done" ? one.dead === "ended" && one.replacedBy === undefined : one.dead === undefined && one.asked === undefined))
          .map((one) => sessionItem(one, hosts)),
      );
    }
    // A failing pull request or red main whose owner is already on it is not a call; one that is, shows its owner.
    const views = viewsOf(state);
    items.push(
      ...lineItems(state, tab, hosts, sessions).flatMap((item): Item[] => {
        const owner = ownerOfCall(views, sessions, item.key);
        if (owner === undefined) return [item];
        return isOnIt(owner, sessions, failedAt(views, item.key)) ? [] : [{ ...item, ownedBy: owner }];
      }),
    );
    return items.filter((item) => !answered.has(item.key));
  }, [tab, state, picked, sessions, slept, walking, answered, dismissed, viewed]);
  // A call dismissed for good stays gone while its state stands; once the state clears, the next one shows.
  const standing = useMemo(() => new Set(items.map(identityOf)), [items]);
  const board = sessions.length > 0 || viewsOf(state).length > 0;
  useEffect(() => {
    if (calls) forgetCleared(standing, board);
  }, [calls, standing, board]);
  return useMemo(() => (calls ? items.filter((item) => !dismissed.has(identityOf(item))) : items), [calls, items, dismissed]);
}
