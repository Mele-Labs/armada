import { useContext, useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent as ReactMouseEvent, ReactNode, Ref, RefObject } from "react";
import { AppWindow, ArrowRight, BadgeCheck, BellRing, Bot, Check, ChevronDown, ChevronRight, ChevronUp, GitBranch, GitPullRequest, KeyRound, Layers, ListChecks, LoaderCircle, MessageSquare, MoveRight, Pencil, Play, Plus, Reply, Scale, Server, Smartphone, SquareTerminal, Trash2, Wrench, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { PathPicker } from "../../primitives/PathPicker/PathPicker";
import { Skeleton, SkeletonText } from "../../primitives/Skeleton/Skeleton";
import { Prose, ProseCommand, ProseRefs, ProseRunner, ProseWalk } from "../../primitives/Prose/Prose";
import type { CodeRunner } from "../../primitives/Prose/Prose";
import { TerminalMark } from "../../primitives/TerminalPane/TerminalPane";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SentTo } from "../HelmThread/SentTo";
import type { ThoughtTarget } from "../ThoughtCard/ThoughtCard";
import { SessionMark } from "../SessionFrame/SessionFrame";
import { SessionStop } from "./SessionStop";
import { TASK_GLYPH, TaskMark, taskMarkSaid, type TaskMarkState } from "../TaskMark/TaskMark";
import { ARTIFACT, PullRequestChecksMark, PullRequestStateMark, type LedgerEntry } from "../SessionLedger/SessionLedger";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";
import { InlineTag, titleWithoutReference } from "../SessionComposer/InlineTag";
import { FORK_ROW, ForkFromSelection } from "../ForkFromSelection/ForkFromSelection";
import type { ForkPick } from "../ForkFromSelection/ForkFromSelection";
import { SessionRetroRow, type SessionRetroRowData } from "../SessionRetroRow/SessionRetroRow";
import { QuestionDeck } from "./QuestionDeck";
import { LinkedRow } from "./LinkedRow";
import { MovedRow } from "./MovedRow";
import { StartedRow } from "./StartedRow";
import { WaiterRow } from "./WaiterRow";
import { SketchRow } from "./SketchRow";
import { PlanRevisionView, useUnchanged } from "./PlanRevision";
import type { PlanRevision } from "./PlanRevision";
export type { PlanRevision, RevisionGroup, RevisionSince, RevisionTask } from "./PlanRevision";
import type { Scene } from "../SketchScene/scene";
import type { ComposerTag } from "../SessionComposer/InlineTag";

/**
 * A Session's conversation, drawn on Helm's own thread rows
 * (`HelmThread.css`) with the two things Helm's thread has no room for.
 *
 * **A message another Session wrote is set apart**: its own band, with the
 * Session mark and the sender's title, over the words, and the band is a press
 * that opens that Session. It is never folded into the agent's voice, which
 * the owner found too easy to mistake it for (6 Oct 2026).
 *
 * **The first write is a row of its own**, filled in the running hue, because
 * it is the one moment a Session stops being blank: the slot leased and the
 * branch cut, in the thread where it happened.
 */
export type SessionThreadRow =
  | {
      id: string;
      at: string;
      kind: "message";
      from: "you" | "agent";
      text: string;
      /** What was sent with it. Pictures are drawn; other files are chips. */
      files?: readonly { id: string; name: string; src?: string }[];
      sketches?: readonly { id: string; title: string }[];
      tags?: readonly ComposerTag[];
      /** The person's message while Fleet has not taken it (`sending`) or did not (`failed`, with a Retry). */
      delivery?: "sending" | "failed";
      /** The key the row keeps while Fleet's own row takes its place, so the message does not draw again. */
      stableKey?: string;
    }
  | { id: string; at: string; kind: "message"; from: "session"; sender: { id: string; title: string }; text: string }
  /** Words typed in Helm's box that went to a Session and never to Helm: Helm's own thread has these, a Session's does not. */
  | {
      id: string;
      at: string;
      kind: "sent";
      text: string;
      target: ThoughtTarget & { id: string };
      files?: readonly { id: string; name: string; src?: string }[];
      sketches?: readonly { id: string; title: string }[];
      tags?: readonly ComposerTag[];
      stableKey?: string;
    }
  /** A tool call, mono. `fromPhone` is an ask the person answered on their phone: a glyph beside it. */
  | { id: string; at: string; kind: "tool"; text: string; fromPhone?: true }
  /** Fleet woke the agent, and why: a quiet row where the turn it started begins. */
  | { id: string; at: string; kind: "woke"; why: string }
  /** What the agent did on its own, mono, with the mark and the tooltip that name which act it was. */
  | { id: string; at: string; kind: "act"; icon: LucideIcon; said: string; text: string }
  /** A command the person ran in the terminal, as typed, mono. */
  | { id: string; at: string; kind: "command"; text: string }
  /** The summary the CLI wrote where it compacted the conversation. Not the person's words: a quiet row that opens to its text. */
  | { id: string; at: string; kind: "compaction"; text: string }
  | { id: string; at: string; kind: "lease"; slot: number; branch: string }
  /** A value the person changed through a panel, which the agent was told of: an event row in his colour. */
  | { id: string; at: string; kind: "change"; what: string; from: string; to: string }
  /** A judge's concern the person dismissed: a settled line, the agent told nothing. */
  | { id: string; at: string; kind: "concern"; step: string; text: string }
  /** The changes and notes the person sent on the plan, his message: a line for each, against its group or task. */
  | { id: string; at: string; kind: "suggest"; lines: readonly { verb: "Renamed" | "Removed" | "Added" | "Moved" | "Subagent" | "Note"; where: string; text?: string; from?: string }[] }
  /** An `armada check` the agent ran: its key and where it stands. A press opens that run on the Checks page. `target` is what the run was narrowed to; absent, it ran whole. */
  | { id: string; at: string; kind: "check"; name: string; run: number; state: CheckRunState; target?: string }
  /** A command the person ran, which the Session was told: the mark of how it ended, the command, and a fold with the lines the Session was sent. `code` is null where it was stopped. */
  | { id: string; at: string; kind: "ran"; command: string; code: number | null; tail: string }
  /** The Session showed a page in a window: a quiet row, a press that opens the window again. */
  | { id: string; at: string; kind: "window"; title: string; url: string }
  /**
   * The agent drew a sketch: a card with a small drawing, a press that opens it. `sketch` is the ledger row's id.
   * `replaced` is set by the host where a later card holds the same `sketch`, and `againstScene` where it finds the card
   * `against` names, so the card draws the change as the panel does.
   */
  | { id: string; at: string; kind: "sketch"; sketch: string; title: string; drawing: Scene; against?: string; againstScene?: Scene; replaced?: true }
  /** This Session's agent started a Session, or forked itself into one: a press that opens it. */
  | { id: string; at: string; kind: "started"; session: { id: string; title: string }; fork: boolean }
  /**
   * A Session that waits on this one, told here: dispatched (`waits`, with the first line of its brief in `says`), its start condition changed (`from` and `to`; `by` is who
   * changed it), or started (`says` is where from). A press opens it.
   */
  | { id: string; at: string; kind: "waiter"; session: { id: string; title: string }; /** Written in the waiting Session's own thread: `session` is the one it waits on. */ waiting?: true; event: "waits" | "changed" | "started"; says?: string; from?: string; to?: string; by?: "you" | "agent" }
  /** A Session this one follows needs the person, finished or merged: the agent was told with it. A press opens that Session. */
  | { id: string; at: string; kind: "linked"; session: { id: string; title: string }; event: "needs" | "done" | "merged" }
  /** The Session was moved to another repository, or is the one that was: a quiet row naming where, a press that opens the other. */
  | { id: string; at: string; kind: "moved"; way: "to" | "from"; session: string; repository: string }
  /** Where an earlier Job's turns end and this one's begin, after a hand off: a plain rule. */
  | { id: string; at: string; kind: "divider" }
  /** A retro being written for this Session, then the one that was: a card that stays in the thread. */
  | SessionRetroRowData;

/** One todo of the agent's list, in the order the band draws it. */
export type WorkingTodo = { key: string; text: string; state: "done" | "doing" | "todo" };

/** Where a Check the agent ran stands. */
export type CheckRunState = "running" | "passed" | "failed";

const CHECK_MARK: Record<CheckRunState, StepActivity> = { running: "running", passed: "advanced", failed: "failed" };

/** One answer an ask will take: what the press hands back, the word on it and what it commits to. */
export type AskOffer = { id: string; label: string; means: string };

/** One question the agent put to the person, with the options it offered. */
export type AskedQuestion = {
  question: string;
  header: string;
  multi_select: boolean;
  options: readonly { label: string; description: string; preview?: string }[];
};

/** What was chosen for one question: option labels, and the person's own words where they chose Other. */
export type QuestionAnswer = { question: string; chosen: string[] };

/**
 * The plan the agent recorded, drawn as the thread draws a question: a card for the person's press while it waits, and once
 * approved a record in the line of the thread, after the row `after` that recorded it, with no press left on it.
 */
export type PlanApproval = {
  /** The version the agent recorded, named on the folded line. */
  version?: number;
  groups: readonly { id: string; agent: string; title: string; tasks: readonly { id: string; text: string }[] }[];
  approved: boolean;
  after?: string;
  /** What the agent changed since the plan before this one: what is added, renamed (and what it was) and removed. */
  changes?: readonly { kind: "added" | "renamed" | "removed"; text: string; from?: string }[];
  /** A later version drawn with what changed marked in place and the groups nothing touched closed to a line. Absent, the card draws the plan whole. */
  revision?: PlanRevision;
  /** A sketch drawn with the plan, shown under the groups as a sketch card is and opened as one is. */
  sketch?: { id: string; title: string; scene: Scene };
  onApprove: () => void;
  /** Opens the plan to read in full: a press on the card anywhere but Approve, and on its sketch. Absent leaves the card plain. */
  onReview?: () => void;
};

/** A concern a judge raised at a step, waiting on the person: drawn as a card the way a question is. */
export type ThreadConcern = {
  id: string;
  step?: string;
  judge?: string;
  /** The refusal as one string, which is what the agent is sent. Drawn on its lines where the parts below are absent. */
  text: string;
  /** The refusal in its parts: what should be seen, what was seen instead, and what the difference does. */
  expected?: string;
  produced?: string;
  consequence?: string;
  cites?: readonly string[];
};

export type SessionThreadProps = {
  rows: readonly SessionThreadRow[];
  /** A plan waiting on the person, or settled. Absent draws none. */
  approval?: PlanApproval;
  /** Sends a message that did not go again, by its row's id. Absent, a message that did not go draws without the press. */
  onRetry?: (rowId: string) => void;
  /**
   * The permission the agent is held on. **`offers` are the answers Fleet will take**, drawn in its
   * order; absent, the card offers Allow once and Deny and names neither. `reaches` is the path past
   * the session's slots it asked about: the card draws it as a folder picker, and an `always_allow`,
   * `always_block` or `allow_for_session` press hands back the folder chosen. `reachesTo` says whether
   * the call reads that path or writes to it, in the card's eyebrow and the Always head.
   */
  asked?: { command: string; offers?: readonly AskOffer[]; questions?: readonly AskedQuestion[]; reaches?: string; reachesTo?: "read" | "write" };
  onAnswer: (answer?: string, answers?: QuestionAnswer[], folder?: string) => void;
  /**
   * A question the agent asked that nobody answered in time, or that was skipped, drawn as the deck under the
   * thread until it is answered or the person says anything else. Not a call: the agent is not held on it, so
   * Skip only puts the deck away, and a live `asked` takes its place.
   */
  lapsed?: { id: string; questions: readonly AskedQuestion[]; onAnswer: (answers: QuestionAnswer[]) => void; onSkip: () => void };
  /**
   * What the agent reports it is working on, from its own todo list: a band of one glyph per todo over
   * the box, the one being done first, which a press opens to the rows. Empty or absent draws none.
   */
  workingOn?: { todos: readonly WorkingTodo[] };
  /** The ask's card, so a host can scroll to it. */
  askRef?: Ref<HTMLDivElement>;
  /** Opens the Session a message came from. */
  onOpenSession: (sessionId: string) => void;
  /** Opens a window the Session showed, again. Absent where windows are not served, and its row is not a press. */
  onOpenWindow?: (url: string) => void;
  /** Opens a sketch the agent drew, by its ledger id. Absent, its card is not a press. */
  onOpenSketch?: (sketch: string) => void;
  /** Opens the Session a row of Helm's sent words went to. Absent, that row's chip is not a press. */
  onOpenSent?: (target: { kind: "session"; id: string }) => void;
  /** Opens a Check run the agent made on the Checks page. Absent, its row is not a press. */
  onOpenCheck?: (run: number) => void;
  /** Opens a retro this Session wrote, by its number, on the Retros page. Absent, the card has no link. Keep it stable: the card is memoised. */
  onOpenRetro?: (retro: number) => void;
  /** Which Session this is. A change of it opens the thread at its newest row again. */
  sessionId?: string;
  /** The turn is running: a live mark sits at the end, on the last tool group's row if that is the last row. */
  working?: boolean;
  /** Ends the running turn: a Stop sits beside the working mark. Absent, none is drawn. */
  onStop?: () => void;
  /** What the agent's own rows are labelled. Absent reads `Agent`. */
  agentName?: string;
  /**
   * Runs a shell block's command where the Session works, from the Run on the block. The person's
   * own act, so nothing is sent to the agent. Absent, a block draws without Run.
   */
  runner?: CodeRunner;
  /**
   * Sends a slash command written as inline code to the Session's terminal, to run as typed. Only a
   * terminal Session has one; absent, such code is drawn plain.
   */
  onCommand?: (command: string) => void;
  /** The names of the commands that terminal lists. Inline code is a command only where its name is one of these. */
  commandNames?: readonly string[];
  /**
   * Forks the Session from the message the person selected words in: a bar over the selection offers it.
   * Absent where nothing serves it, and the bar is left off.
   */
  onForkAt?: (pick: ForkPick) => void;
  /**
   * Replies to the words the person selected in a message: the bar offers it beside Fork. The caller
   * puts the quote in the message box. Absent where there is no box, and the act is left off.
   */
  onReplyAt?: (pick: ForkPick) => void;
  /** Drawn first in the thread, a row like the rest: it scrolls away with them. */
  lead?: ReactNode;
  /** `unread`: no rows held yet, and a transcript's shape stands in for them. `refreshing`: the rows held are being read again. */
  transcript?: "unread" | "refreshing";
};

const TAG_KIND = { session: "Job", pull_request: "Pull request", branch: "Branch", issue: "Issue" } as const;

/**
 * A sent message's words with each tag drawn as the chip it was written as, at its `@title`. `parts`
 * is `null` where no tag is found in the words, and `loose` holds the tags that were not.
 */
function placeTags(text: string, tags: readonly ComposerTag[], open?: (tag: ComposerTag) => void): { parts: ReactNode[] | null; loose: readonly ComposerTag[] } {
  const parts: ReactNode[] = [];
  const loose: ComposerTag[] = [];
  let from = 0;
  for (const tag of tags) {
    const at = text.indexOf(`@${tag.title}`, from);
    if (at < 0) {
      loose.push(tag);
      continue;
    }
    if (at > from) parts.push(text.slice(from, at));
    parts.push(<InlineTag key={`${from}${tag.kind}${tag.id}`} tag={tag} {...(open === undefined ? {} : { onOpen: () => open(tag) })} />);
    from = at + tag.title.length + 1;
  }
  if (parts.length === 0) return { parts: null, loose };
  if (from < text.length) parts.push(text.slice(from));
  return { parts, loose };
}

function Sent({ row, tags }: { row: { files?: readonly { id: string; name: string; src?: string }[]; sketches?: readonly { id: string; title: string }[] }; tags: readonly ComposerTag[] }) {
  const files = row.files ?? [];
  const sketches = row.sketches ?? [];
  if (files.length + sketches.length + tags.length === 0) return null;
  return (
    <div className="armada-session-sent">
      {files.map((file) =>
        file.src === undefined ? (
          <AttachmentChip key={file.id} filename={file.name} />
        ) : (
          <img key={file.id} className="armada-session-sent__picture" src={file.src} alt={file.name} />
        ),
      )}
      {sketches.map((one) => (
        <AttachmentChip key={one.id} filename={one.title} from="Sketch" />
      ))}
      {tags.map((one) => (
        <AttachmentChip key={`${one.kind}${one.id}`} filename={one.title} from={TAG_KIND[one.kind]} />
      ))}
    </div>
  );
}

/** Helm's thread draws a sent message the way a Session's does. */
export { Sent as SentWith, placeTags as placeTagsIn };

type CheckRow = Extract<SessionThreadRow, { kind: "check" }>;

/**
 * What the thread draws: a row, the tool calls that ran one after another folded into one, or the
 * runs of one Check that sat side by side folded into one. A Check run beside another Check, or
 * after a tool call, starts a fold of its own.
 */
type Item =
  | { kind: "row"; row: Exclude<SessionThreadRow, { kind: "tool" | "check" }> }
  | { kind: "calls"; id: string; rows: readonly Extract<SessionThreadRow, { kind: "tool" }>[] }
  | { kind: "checks"; id: string; rows: readonly CheckRow[] };

function itemsOf(rows: readonly SessionThreadRow[]): Item[] {
  const items: Item[] = [];
  for (const row of rows) {
    const last = items[items.length - 1];
    if (row.kind === "check") {
      if (last?.kind === "checks" && last.rows[0]?.name === row.name) last.rows = [...last.rows, row];
      else items.push({ kind: "checks", id: row.id, rows: [row] });
    } else if (row.kind !== "tool") items.push({ kind: "row", row });
    else if (last?.kind === "calls") last.rows = [...last.rows, row];
    else items.push({ kind: "calls", id: row.id, rows: [row] });
  }
  return items;
}

/** The tools a group holds, each named once, in the order they were first called. */
function namesOf(rows: readonly { text: string }[]): string[] {
  return [...new Set(rows.map((one) => one.text.split(/\s/, 1)[0] ?? ""))].filter((name) => name !== "");
}

/**
 * **Calls that ran one after another are one row, closed.** It names the tools it holds and draws no
 * count; pressing it shows each call. A lone call is a group of one.
 */
function Calls({ rows, working, onStop }: { rows: readonly { id: string; text: string; fromPhone?: true }[]; working?: boolean; onStop?: () => void }) {
  const names = namesOf(rows);
  return (
    <li className="armada-session-fold armada-session-fold--calls">
      <details>
        <summary className="armada-session-fold__head" aria-label={`Tool calls: ${names.join(", ")}`}>
          <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-fold__chevron" />
          {working === true ? <SessionMark state="working" said="Working" /> : <Wrench size={12} strokeWidth={2} aria-hidden />}
          {working === true && onStop !== undefined ? <SessionStop onStop={onStop} /> : null}
          <span className="armada-session-fold__names">{names.join(", ")}</span>
        </summary>
        <ol className="armada-session-fold__calls">
          {rows.map((one) => (
            <li key={one.id} className="armada-session-command">
              <code className="armada-session-command__text">{one.text}</code>
              {one.fromPhone === true ? (
                <Tooltip label="From phone">
                  <span className="armada-session-command__door">
                    <Smartphone size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              ) : null}
            </li>
          ))}
        </ol>
      </details>
    </li>
  );
}

function CheckMark({ row }: { row: CheckRow }) {
  return <StepActivityMark activity={CHECK_MARK[row.state]} label={row.state} says={row.state} pulsing={row.state === "running"} />;
}

const checkSaid = (row: CheckRow) => `Check ${row.name}${row.target === undefined ? "" : ` ${row.target}`}, ${row.state}`;

/**
 * **Runs of one Check that sat side by side are one row, closed.** It shows the latest run's mark and the
 * Check's name, and draws no count; opened, each run is a line with what it was narrowed to. A lone run
 * is a group of one, drawn as a press with its target beside the name.
 */
function CheckRuns({ rows, onOpenCheck }: { rows: readonly CheckRow[]; onOpenCheck?: (run: number) => void }) {
  const latest = rows[rows.length - 1]!;
  if (rows.length === 1) {
    const inner = (
      <>
        <CheckMark row={latest} />
        <span className="mono">{latest.name}</span>
        {latest.target === undefined ? null : <code className="armada-session-fold__target">{latest.target}</code>}
      </>
    );
    return (
      <li className="armada-session-fold">
        {onOpenCheck === undefined ? (
          <span className="armada-session-fold__head">{inner}</span>
        ) : (
          <button type="button" className="armada-session-fold__head armada-session-fold__head--press" aria-label={checkSaid(latest)} onClick={() => onOpenCheck(latest.run)}>
            {inner}
          </button>
        )}
      </li>
    );
  }
  return (
    <li className="armada-session-fold armada-session-fold--checks">
      <details>
        <summary className="armada-session-fold__head" aria-label={`Check ${latest.name}, latest ${latest.state}`}>
          <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-fold__chevron" />
          <CheckMark row={latest} />
          <span className="mono">{latest.name}</span>
        </summary>
        <ol className="armada-session-fold__calls">
          {rows.map((one) => {
            const inner = (
              <>
                <CheckMark row={one} />
                {one.target === undefined ? null : <code className="armada-session-fold__target">{one.target}</code>}
                <span className="armada-session-fold__at">{one.at}</span>
              </>
            );
            return (
              <li key={one.id}>
                {onOpenCheck === undefined ? (
                  <span className="armada-session-fold__head">{inner}</span>
                ) : (
                  <button type="button" className="armada-session-fold__head armada-session-fold__head--press" aria-label={checkSaid(one)} onClick={() => onOpenCheck(one.run)}>
                    {inner}
                  </button>
                )}
              </li>
            );
          })}
        </ol>
      </details>
    </li>
  );
}

/**
 * What the person did, as Fleet sends it to the agent in their name: `Approved: <address>` from a
 * walk's Approve, and `Re: <item>` over the answer from a waiting item. Anything else is their words.
 */
type Act = { kind: "approved"; target: string; shown: string } | { kind: "answered"; target: string; said: string };

/** Fleet's last line to the agent on an answer (`TAKEN_OFF` in waiting.rs), then and now: not the person's words. */
const TAKEN_OFF = /\n?(?:Fleet took this off your waiting list\.|This settles the item, so Fleet took it off your waiting list\. Do not add it back\.)$/;

function actOf(text: string): Act | null {
  const approved = /^Approved: (\S+)$/.exec(text);
  if (approved?.[1] !== undefined) {
    const walk = /[?&]walk=[^&#\s]*/.exec(approved[1]);
    return { kind: "approved", target: approved[1], shown: walk === null ? approved[1] : `?${walk[0].slice(1)}` };
  }
  const answered = /^Re: ([^\n]+)\n?([\s\S]*)$/.exec(text);
  if (answered?.[1] !== undefined) return { kind: "answered", target: answered[1], said: (answered[2] ?? "").replace(TAKEN_OFF, "").trim() };
  return null;
}

/** A target this long is likely cut to one line, so it names itself in a tooltip. */
const TARGET_TRUNCATES = 40;

/** The person's act: one filled line in the inverse neutral, what it was and on what. */
function Did({ act, at }: { act: Act; at: string }) {
  const approved = act.kind === "approved";
  const Glyph = approved ? Check : Reply;
  const word = approved ? "Approved" : "Answered";
  const shown = approved ? act.shown : act.target;
  const chip = (
    <span className="armada-session-did__chip" role="img" aria-label={`${word}: ${act.target}`}>
      <Glyph size={12} strokeWidth={2} aria-hidden />
      <span className="armada-session-did__target">{shown}</span>
    </span>
  );
  return (
    <li className="armada-session-did" role="region" aria-label={`You ${word.toLowerCase()}`}>
      <span className="armada-session-did__eyebrow">{word}</span>
      {shown !== act.target || act.target.length > TARGET_TRUNCATES ? <Tooltip label={act.target} asChild>{chip}</Tooltip> : chip}
      {act.kind === "answered" && act.said !== "" ? <span className="armada-session-did__said">{act.said}</span> : null}
      <span className="armada-session-did__at">{at}</span>
    </li>
  );
}

/** Where a message of the person's stands while Fleet has not taken it: turning while it goes, a cross where it did not. Nothing once it is Fleet's row. */
function Delivery({ row }: { row: { delivery?: "sending" | "failed" } }) {
  if (row.delivery === undefined) return null;
  const sending = row.delivery === "sending";
  const said = sending ? "Sending" : "Not sent";
  return (
    <Tooltip label={said}>
      <span className="armada-session-you__delivery" data-state={row.delivery} role="img" aria-label={said}>
        {sending ? <LoaderCircle size={12} strokeWidth={2} aria-hidden /> : <X size={12} strokeWidth={2} aria-hidden />}
      </span>
    </Tooltip>
  );
}

/** What a message of the person's draws from. */
type YourRow = Pick<Extract<SessionThreadRow, { kind: "message"; from: "you" | "agent" }>, "id" | "at" | "text" | "files" | "sketches" | "tags" | "delivery">;

/**
 * The person's own message: the band with its Delivery mark, the words, and a Retry where it did not go.
 * **One row wherever words are sent**: a Session's thread, Helm's, and the box under a Drone's log draw this.
 */
export function YourMessage({ row, onRetry, forkable = false }: { row: YourRow; onRetry?: (rowId: string) => void; forkable?: boolean }) {
  // Where the host can open what a chip in a row names (Helm's thread links them), a tag is a link.
  const refs = useContext(ProseRefs);
  const openTag = refs === null ? undefined : (tag: ComposerTag): void => (tag.kind === "branch" ? undefined : refs.open({ kind: tag.kind, id: tag.id, title: titleWithoutReference(tag) }));
  const placed = placeTags(row.text, row.tags ?? [], openTag);
  return (
    <li className="armada-session-you" role="region" aria-label="Your message" data-delivery={row.delivery}>
      <div className="armada-session-you__band">
        <span className="armada-session-you__who">You</span>
        <Delivery row={row} />
        <span className="armada-session-you__at">{row.at}</span>
      </div>
      <div className="armada-session-you__body" {...(forkable ? { [FORK_ROW]: row.id } : {})}>
        {placed.parts !== null ? (
          <p className="armada-session-said">{placed.parts}</p>
        ) : (
          // Run is on what the agent wrote, not on what the person did.
          <ProseRunner.Provider value={null}>
            <ProseCommand.Provider value={null}>
              <Prose text={row.text} />
            </ProseCommand.Provider>
          </ProseRunner.Provider>
        )}
        <Sent row={row} tags={placed.loose} />
        {row.delivery === "failed" && onRetry !== undefined ? (
          <div className="armada-session-you__retry">
            <Button size="sm" variant="secondary" onClick={() => onRetry(row.id)}>
              Retry
            </Button>
          </div>
        ) : null}
      </div>
    </li>
  );
}

function Row({
  row,
  onOpenSession,
  onOpenWindow,
  onOpenSketch,
  agentName = "Agent",
  onOpenRetro,
  onOpenSent,
  onRetry,
  forkable,
}: {
  row: Exclude<SessionThreadRow, { kind: "tool" | "check" }>;
  onRetry?: (rowId: string) => void;
  forkable: boolean;
  onOpenSession: (id: string) => void;
  onOpenWindow?: (url: string) => void;
  onOpenSketch?: (sketch: string) => void;
  agentName?: string;
  onOpenRetro?: (retro: number) => void;
  onOpenSent?: (target: { kind: "session"; id: string }) => void;
}) {
  // Where the host can open what a chip in a row names (Helm's thread links them), a tag is a link.
  const refs = useContext(ProseRefs);
  const openTag = refs === null ? undefined : (tag: ComposerTag): void => (tag.kind === "branch" ? undefined : refs.open({ kind: tag.kind, id: tag.id, title: titleWithoutReference(tag) }));
  if (row.kind === "retro") return <SessionRetroRow row={row} {...(onOpenRetro === undefined ? {} : { onOpen: onOpenRetro })} />;
  if (row.kind === "woke") {
    return (
      <li className="armada-session-woke">
        <Tooltip label="Fleet woke it">
          <span className="armada-session-woke__mark" role="img" aria-label="Fleet woke it">
            <Server size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-session-woke__why">{row.why}</span>
        <span className="armada-session-woke__at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "act") {
    const Glyph = row.icon;
    return (
      <li className="armada-session-act">
        <Tooltip label={row.said}>
          <span className="armada-session-act__mark" role="img" aria-label={row.said}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <code className="armada-session-act__text">{row.text}</code>
        <span className="armada-session-act__at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "window") {
    return (
      <li className="armada-session-fold">
        <button type="button" className="armada-session-fold__head armada-session-fold__head--press" aria-label={`Open window ${row.title}`} onClick={() => onOpenWindow?.(row.url)}>
          <AppWindow size={12} strokeWidth={2} aria-hidden />
          {row.title}
        </button>
      </li>
    );
  }
  if (row.kind === "ran") {
    const phase = { state: "ended", code: row.code } as const;
    const first = row.command.split("\n")[0] ?? "";
    return (
      <li className="armada-session-fold armada-session-fold--ran">
        <details>
          <summary className="armada-session-fold__head" aria-label={`${row.code === null ? "Stopped" : `Exit ${row.code}`}: ${first}`}>
            <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-fold__chevron" />
            <TerminalMark phase={phase} />
            <code className="armada-session-fold__target">{first}</code>
          </summary>
          <pre className="armada-session-fold__tail" role="log" aria-label="Sent to the Job">
            {row.tail}
          </pre>
        </details>
      </li>
    );
  }
  if (row.kind === "sketch") return <SketchRow row={row} {...(onOpenSketch === undefined ? {} : { onOpen: onOpenSketch })} />;
  if (row.kind === "started") return <StartedRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "moved") return <MovedRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "divider") return <li className="armada-session-divider" role="separator" />;
  if (row.kind === "waiter") return <WaiterRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "linked") return <LinkedRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "change") {
    return (
      <li className="armada-session-change" role="region" aria-label={`${row.what} changed`}>
        <span className="armada-session-lease__eyebrow">{row.what}</span>
        <span className="armada-session-change__facts">
          {row.from}
          <ArrowRight size={12} strokeWidth={2} aria-hidden />
          {row.to}
        </span>
        <span className="armada-session-lease__at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "concern") {
    return (
      <li className="armada-session-concern-settled" role="region" aria-label="Concern dismissed">
        <Tooltip label="Judge concern, dismissed">
          <span role="img" aria-label="Judge concern, dismissed" className="armada-session-concern-settled__mark">
            <Scale size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <code>{row.step}</code>
        <span className="armada-session-concern-settled__text">{row.text}</span>
        <span className="armada-session-lease__at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "suggest") {
    const glyphs = { Renamed: Pencil, Removed: Trash2, Added: Plus, Moved: MoveRight, Subagent: Bot, Note: MessageSquare } as const;
    return (
      <li className="armada-session-suggest" role="region" aria-label="Changes to the plan">
        <span className="armada-session-lease__eyebrow">Plan changes</span>
        <ul className="armada-session-suggest__lines">
          {row.lines.map((line, at) => {
            const Glyph = glyphs[line.verb];
            return (
              <li key={at} className="armada-session-suggest__line">
                <Tooltip label={line.verb}>
                  <span role="img" aria-label={line.verb} className="armada-session-suggest__verb">
                    <Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
                <span className="armada-session-suggest__where">{line.where}</span>
                {line.from === undefined ? null : <s>{line.from}</s>}
                {line.from !== undefined && line.text !== undefined ? <ArrowRight size={12} strokeWidth={2} aria-hidden /> : null}
                {line.text === undefined ? null : <span className="armada-session-suggest__text">{line.text}</span>}
              </li>
            );
          })}
        </ul>
        <span className="armada-session-lease__at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "lease") {
    return (
      <li className="armada-session-lease" role="region" aria-label="Leased on first write">
        <span className="armada-session-lease__eyebrow">First write</span>
        <span className="armada-session-lease__facts">
          <span className="armada-session-lease__chip" role="img" aria-label={`Worktree slot ${row.slot}`}>
            <KeyRound size={12} strokeWidth={2} aria-hidden />
            {row.slot}
          </span>
          <span className="armada-session-lease__chip" role="img" aria-label={`Branch ${row.branch}`}>
            <GitBranch size={12} strokeWidth={2} aria-hidden />
            {row.branch}
          </span>
        </span>
        <span className="armada-session-lease__at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "command") {
    return (
      <li className="armada-session-command">
        <code className="armada-session-command__text">{row.text}</code>
      </li>
    );
  }
  if (row.kind === "compaction") {
    return (
      <li className="armada-session-fold">
        <details>
          <summary className="armada-session-fold__head">
            <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-fold__chevron" />
            <Layers size={12} strokeWidth={2} aria-hidden />
            Conversation compacted
          </summary>
          <div className="armada-session-fold__body">
            <Prose text={row.text} />
          </div>
        </details>
      </li>
    );
  }
  if (row.kind === "sent") {
    const placed = placeTags(row.text, row.tags ?? [], openTag);
    return (
      <li className="armada-session-you" role="region" aria-label="Your message">
        <div className="armada-session-you__band">
          <span className="armada-session-you__who">You</span>
          <span className="armada-session-you__at">{row.at}</span>
        </div>
        <div className="armada-session-you__body">
          {placed.parts !== null ? <p className="armada-session-said">{placed.parts}</p> : <Prose text={row.text} />}
          <Sent row={row} tags={placed.loose} />
          <SentTo target={row.target} onOpen={() => onOpenSent?.({ kind: row.target.kind, id: row.target.id })} />
        </div>
      </li>
    );
  }
  if (row.from === "session") {
    const { sender } = row;
    return (
      <li className="armada-session-from" role="region" aria-label={`Message from ${sender.title}`}>
        <button type="button" className="armada-session-from__band" aria-label={`Open Job ${sender.title}`} onClick={() => onOpenSession(sender.id)}>
          <SquareTerminal size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-from__id">{sender.id}</span>
          <span className="armada-session-from__title">{sender.title}</span>
          <span className="armada-session-from__at">{row.at}</span>
        </button>
        <div className="armada-session-from__body" {...(forkable ? { [FORK_ROW]: row.id } : {})}>
          <Prose text={row.text} />
        </div>
      </li>
    );
  }
  if (row.from === "you") {
    const act = actOf(row.text);
    if (act !== null) return <Did act={act} at={row.at} />;
    return <YourMessage row={row} forkable={forkable} {...(onRetry === undefined ? {} : { onRetry })} />;
  }
  const placed = placeTags(row.text, row.tags ?? [], openTag);
  return (
    <li className="armada-helm-thread__row" data-actor="helm" data-from={row.from}>
      <div className="armada-helm-thread__head">
        <span className="armada-helm-thread__who">{agentName}</span>
        <span className="armada-helm-thread__at">{row.at}</span>
      </div>
      <div className="armada-helm-thread__message" {...(forkable ? { [FORK_ROW]: row.id } : {})}>
        {placed.parts !== null ? <p className="armada-session-said">{placed.parts}</p> : <Prose text={row.text} />}
      </div>
      <Sent row={row} tags={placed.loose} />
    </li>
  );
}

/** A thread not read yet: the shape of the person's message and the agent's reply, twice, hidden from a screen reader as the list says it is busy. */
function TranscriptSkeleton() {
  return (
    <li className="armada-session-thread__skeleton" aria-hidden>
      {[["70%", "45%"], ["55%"]].map((said, at) => (
        <div key={at} className="armada-session-thread__skeleton-turn">
          <div className="armada-session-thread__skeleton-you">
            <SkeletonText widths={said} />
          </div>
          <div className="armada-session-thread__skeleton-agent">
            <Skeleton width="15%" />
            <SkeletonText widths={at === 0 ? ["92%", "80%", "86%", "40%"] : ["88%", "64%"]} />
          </div>
        </div>
      ))}
    </li>
  );
}

/** How far from the end still counts as being at it, so a rounding or a half row does not unpin. */
const NEAR_END = 24;
/** How many items a thread draws at first, and how many more each press of Show earlier adds. A Session that ran for days is thousands of rows, and drawing them all makes typing beside it lag. */
const DRAWN = 150;

/** Words in a field the person types in are theirs to keep; only the thread's own text is copied. */
const EDITABLE = "textarea, input, [contenteditable]:not([contenteditable='false'])";

/**
 * Words selected with the pointer inside `scope` go to the clipboard when the selection ends, silently.
 * **It reads the selection and never owns it**, as `ForkFromSelection` does, so the bar and the
 * selection are left as they were.
 */
function useCopyOnSelect(scope: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const home = scope.current;
    if (home === null) return;
    let dragging = false;
    const down = (event: MouseEvent) => {
      if (event.button === 0) dragging = true;
    };
    const up = () => {
      if (!dragging) return;
      dragging = false;
      const selection = window.getSelection();
      if (selection === null || selection.rangeCount === 0 || selection.isCollapsed) return;
      const around = selection.getRangeAt(0).commonAncestorContainer;
      const at = around instanceof Element ? around : around.parentElement;
      if (at === null || !home.contains(at) || at.closest(EDITABLE) !== null) return;
      const active = document.activeElement;
      if (active !== null && active.matches(EDITABLE) && home.contains(active)) return;
      const words = selection.toString();
      if (words !== "") void navigator.clipboard.writeText(words).catch(() => undefined);
    };
    home.addEventListener("mousedown", down);
    document.addEventListener("mouseup", up);
    return () => {
      home.removeEventListener("mousedown", down);
      document.removeEventListener("mouseup", up);
    };
  }, [scope]);
}

/** The answers that carry a folder, and so the picker's choice. */
const FOLDER_ANSWERS = new Set(["always_allow", "allow_for_session", "always_block"]);

/** The Always head, read with the path under it: "reads of" a path, "writes to" one. */
const ALWAYS_HEAD = { read: ["reads", "of"], write: ["writes", "to"] } as const;

/** A card's answers: the ones for this call in a row, then **Always** in a panel of its own, the
 * folder picker inside it so the choice reads as what Always allow and Always block cover. Keyed
 * by the path, so a new ask opens on its whole path. */
function ReachAnswers({ reaches, reachesTo, offers, onAnswer }: { reaches: string; reachesTo?: "read" | "write"; offers: readonly AskOffer[]; onAnswer: SessionThreadProps["onAnswer"] }) {
  const [folder, setFolder] = useState(reaches);
  const shown = offers.filter((offer) => offer.id !== "allow_once");
  const once = shown.filter((offer) => !FOLDER_ANSWERS.has(offer.id));
  const always = shown.filter((offer) => FOLDER_ANSWERS.has(offer.id));
  const name = folder.split("/").filter((part) => part.length > 0).pop() ?? folder;
  const covers = folder === reaches ? reaches : `${folder}/*`;
  return (
    <div className="armada-session-thread__reach" role="group" aria-label="Answers">
      <div className="armada-session-thread__answers">
        {once.map((offer) => (
          <Tooltip key={offer.id} label={offer.means}>
            <Button size="sm" variant="secondary" onClick={() => onAnswer(offer.id)}>
              {offer.label}
            </Button>
          </Tooltip>
        ))}
      </div>
      <div className="armada-session-thread__always">
        <span className="armada-session-thread__always-head">
          {reachesTo === undefined ? (
            "Always applies to"
          ) : (
            <>
              Always applies to <strong className="armada-session-thread__always-kind">{ALWAYS_HEAD[reachesTo][0]}</strong> {ALWAYS_HEAD[reachesTo][1]}
            </>
          )}
        </span>
        <PathPicker path={reaches} chosen={folder} onChoose={setFolder} label="Folder for always" />
        <span className="armada-session-thread__always-covers">{covers}</span>
        <div className="armada-session-thread__answers">
          {always.map((offer) => (
            <Tooltip key={offer.id} label={offer.means}>
              <Button size="sm" variant="secondary" onClick={() => onAnswer(offer.id, undefined, folder)}>
                {offer.label} <span className="armada-session-thread__always-folder">{name}</span>
              </Button>
            </Tooltip>
          ))}
        </div>
      </div>
    </div>
  );
}

/** A permission that offers Allow once draws its command as a block to run, and ▶ is that offer. */
/**
 * The plan as a compact card: each group a closed row (its subagent type, its title and a mark for each task), a small thumbnail of
 * the sketch drawn with it, and Approve. **A press anywhere on it but Approve opens the plan in full** (`onReview`), where every group is
 * open and the sketch is large. Short enough that the thread never has to scroll it under the box.
 */
function PlanCard({ approval }: { approval: PlanApproval }) {
  const { approved, onReview } = approval;
  const [folded, setFolded] = useState(false);
  const Fold = folded ? ChevronRight : ChevronDown;
  const revising = approval.revision !== undefined && !approved;
  const unchanged = useUnchanged(revising ? approval.revision : undefined);
  const stateSaid = approved ? "Plan approved" : "Plan waiting for approval";
  const StateGlyph = approved ? BadgeCheck : BellRing;
  const review = onReview === undefined || folded ? undefined : { tabIndex: 0, "data-reviewable": "", onClick: (event: ReactMouseEvent<HTMLElement>) => { if ((event.target as Element).closest("button") === null) onReview(); }, onKeyDown: (event: KeyboardEvent<HTMLElement>) => { if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); onReview(); } } };
  // U opens the unchanged groups of a revision, or closes them all, wherever focus is on the card.
  const onKeys = (event: KeyboardEvent<HTMLElement>) => {
    if (!folded && event.key === "u" && !event.metaKey && !event.ctrlKey && !event.altKey) {
      event.preventDefault();
      unchanged.toggleAll();
    } else review?.onKeyDown(event);
  };
  return (
    <Card flat className="armada-session-thread__ask armada-session-thread__plan" role="article" aria-label={approved ? "Plan approved" : "Plan to approve"} data-approved={approved || undefined} data-folded={folded || undefined} {...review} {...(revising ? { onKeyDown: onKeys } : {})}>
      <div className="armada-session-thread__plan-head">
        <button type="button" className="armada-session-thread__plan-band" aria-expanded={!folded} aria-label="Plan" onClick={() => setFolded(!folded)}>
          <ListChecks size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-thread__eyebrow">Plan</span>
          {approval.version === undefined ? null : <span className="armada-session-thread__plan-version">v{approval.version}</span>}
          <Tooltip label={stateSaid}>
            <span className="armada-session-thread__settled" role="img" aria-label={stateSaid} data-state={approved ? "approved" : "waiting"}>
              <StateGlyph size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <Fold className="armada-session-thread__plan-fold" size={12} strokeWidth={2} aria-hidden />
        </button>
        {approved || !folded ? null : (
          <Button size="sm" variant="primary" onClick={approval.onApprove}>
            Approve
          </Button>
        )}
      </div>
      {folded ? null : (
        <>
      {revising || approval.changes === undefined || approval.changes.length === 0 ? null : (
        <ul className="armada-session-thread__changes" aria-label="Changed since the last plan">
          {approval.changes.map((one, at) => (
            <li key={at} data-kind={one.kind}>
              <Tooltip label={one.kind === "added" ? "Added" : one.kind === "renamed" ? "Renamed" : "Removed"}>
                <span role="img" aria-label={one.kind === "added" ? "Added" : one.kind === "renamed" ? "Renamed" : "Removed"}>
                  {one.kind === "added" ? <Plus size={12} strokeWidth={2} aria-hidden /> : one.kind === "renamed" ? <Pencil size={12} strokeWidth={2} aria-hidden /> : <Trash2 size={12} strokeWidth={2} aria-hidden />}
                </span>
              </Tooltip>
              <span className="armada-session-thread__change-text">
                {one.from === undefined ? null : <s>{one.from}</s>}
                {one.kind === "removed" ? <s>{one.text}</s> : <span>{one.text}</span>}
              </span>
            </li>
          ))}
        </ul>
      )}
      <div className="armada-session-thread__plan-body">
        {revising ? <PlanRevisionView revision={approval.revision!} unchanged={unchanged} /> : (
        <ul className="armada-session-thread__groups">
          {approval.groups.map((group) => (
            <li key={group.id} className="armada-session-thread__group">
              <span className="armada-session-thread__group-title">{group.title}</span>
              {approved ? null : (
                <span className="armada-session-thread__marks">
                  {group.tasks.map((task) => (
                    <TaskMark key={task.id} state="open" />
                  ))}
                </span>
              )}
              <Tooltip label="Subagent type">
                <span className="armada-session-thread__agent" role="img" aria-label={`Subagent type ${group.agent}`}>
                  <Bot size={12} strokeWidth={2} aria-hidden />
                  <code>{group.agent}</code>
                </span>
              </Tooltip>
            </li>
          ))}
        </ul>
        )}
        {approval.sketch === undefined ? null : (
          <ul className="armada-session-thread__sketch">
            <SketchRow row={{ id: approval.sketch.id, at: "", kind: "sketch", sketch: approval.sketch.id, title: approval.sketch.title, drawing: approval.sketch.scene }} onOpen={() => onReview?.()} />
          </ul>
        )}
      </div>
      {approved ? null : (
        <div className="armada-session-thread__answers" role="group" aria-label="Answers">
          <Button size="sm" variant="primary" onClick={approval.onApprove}>
            Approve
          </Button>
        </div>
      )}
        </>
      )}
    </Card>
  );
}

/**
 * A judge's concern as a claim to read: the consequence over everything, then what was expected and what was found, each a label and a
 * sentence clamped to two lines with a More press for the rest. A concern kept without its parts is its `text`, a line to a paragraph.
 */
function ConcernClaim({ concern }: { concern: ThreadConcern }) {
  const { expected, produced, consequence } = concern;
  if (expected === undefined && produced === undefined && consequence === undefined) {
    return (
      <div className="armada-session-thread__claim">
        {concern.text.split("\n").filter((line) => line.trim() !== "").map((line, at) => (
          <p key={at} className="armada-session-thread__claim-text">{line}</p>
        ))}
      </div>
    );
  }
  return (
    <div className="armada-session-thread__claim">
      {consequence === undefined ? null : <p className="armada-session-thread__claim-head">{consequence}</p>}
      {expected === undefined ? null : <ClaimRow label="Expected" text={expected} />}
      {produced === undefined ? null : <ClaimRow label="Found" text={produced} />}
    </div>
  );
}

function ClaimRow({ label, text }: { label: string; text: string }) {
  const value = useRef<HTMLParagraphElement>(null);
  const [open, setOpen] = useState(false);
  const [clamped, setClamped] = useState(false);
  const id = useId();
  // More is offered only where the two lines leave words out: measured while it is closed, kept while it is open.
  useLayoutEffect(() => {
    const one = value.current;
    if (one === null || open) return;
    const measure = () => setClamped(one.scrollHeight > one.clientHeight + 1);
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(one);
    return () => watch.disconnect();
  }, [text, open]);
  return (
    <div className="armada-session-thread__claim-row">
      <span className="armada-session-thread__eyebrow">{label}</span>
      <p ref={value} id={id} className="armada-session-thread__claim-text" data-open={open || undefined}>{text}</p>
      {clamped || open ? (
        <Button size="sm" variant="ghost" aria-expanded={open} aria-controls={id} aria-label={`${open ? "Less" : "More"} of ${label.toLowerCase()}`} onClick={() => setOpen(!open)}>
          {open ? "Less" : "More"}
        </Button>
      ) : null}
    </div>
  );
}

const allowsOnce = (asked: NonNullable<SessionThreadProps["asked"]>) => asked.offers?.some((offer) => offer.id === "allow_once") === true;

/**
 * The agent says it has nothing left, and the two ways on. `made` are ledger entries (pull requests, or artifacts), listed as presses; none draws no list.
 * A pull request's entry carries `pr`: its state and Checks, each a bare mark with a tooltip.
 */
export type ThreadDone = { onEnd: () => void; onKeep: () => void; made?: readonly LedgerEntry[] };

/** What the agent made, and the two ways on: End session and Keep going. The body of the Session end panel. */
export function DoneBody({ done }: { done: ThreadDone }) {
  return (
    <>
          {done.made === undefined || done.made.length === 0 ? null : (
            <ul className="armada-session-done__made" aria-label={done.made[0]!.kind === "pull_request" ? "Pull requests" : "Artifacts"}>
              {done.made.map((row) => {
                const form = row.artifact === undefined ? undefined : ARTIFACT[row.artifact];
                return (
                  <li key={row.key} className="armada-session-ledger__row" aria-label={row.name}>
                    <button type="button" className="armada-session-ledger__open" aria-label={`Open ${row.name}`} onClick={row.onOpen}>
                      {row.pr !== undefined ? (
                        <PullRequestStateMark merged={row.pr.merged} />
                      ) : form === undefined ? (
                        <GitPullRequest size={12} strokeWidth={2} aria-hidden />
                      ) : (
                        <Tooltip label={form.said}>
                          <span className="armada-session-mark" role="img" aria-label={form.said}>
                            <form.Glyph size={12} strokeWidth={2} aria-hidden />
                          </span>
                        </Tooltip>
                      )}
                      <span className="armada-session-ledger__text">{row.text}</span>
                      {row.pr === undefined ? null : <PullRequestChecksMark checks={row.pr.checks} said={row.pr.checksSaid} />}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
          <div className="armada-session-done__acts">
            <Button size="sm" variant="secondary" onClick={done.onEnd}>
              End job
            </Button>
            <Button size="sm" variant="ghost" onClick={done.onKeep}>
              Keep going
            </Button>
          </div>
    </>
  );
}

/** A judge's concern as a card: where it came from, the claim, what it cites, and the two ways on. `bare` leaves the band off, for a panel that already says what it is. */
export function ConcernCard({ concern: one, onSend, onDismiss, bare }: { concern: ThreadConcern; onSend?: () => void; onDismiss?: () => void; bare?: true }) {
  return (
    <Card flat className="armada-session-thread__ask armada-session-thread__concern" role="group" aria-label={`Judge: ${one.text}`}>
      {bare === undefined ? (
        <div className="armada-session-thread__concern-band">
          <Tooltip label="Judge">
            <span role="img" aria-label="Judge" className="armada-session-thread__concern-judge">
              <Scale size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span className="armada-session-thread__eyebrow">Judge</span>
          <span className="armada-session-thread__concern-from">{one.step ?? "Judge tile"}</span>
        </div>
      ) : null}
      <ConcernClaim concern={one} />
      {one.cites === undefined || one.cites.length === 0 ? null : (
        <ul className="armada-session-thread__cites" aria-label="Cites">
          {one.cites.map((path) => (
            <li key={path}>
              <code>{path}</code>
            </li>
          ))}
        </ul>
      )}
      <div className="armada-session-thread__answers" role="group" aria-label="Answers">
        <Tooltip label="Goes to the agent as your message">
          <Button size="sm" variant="secondary" onClick={onSend}>
            Send to job
          </Button>
        </Tooltip>
        <Button size="sm" variant="ghost" onClick={onDismiss}>
          Dismiss
        </Button>
      </div>
    </Card>
  );
}

/** The words on the pill that holds the question: its header, or the permission card's own eyebrow. */
export function askLabel({ asked, lapsed }: Pick<SessionThreadProps, "asked" | "lapsed">): string {
  if (asked !== undefined && (asked.questions?.length ?? 0) === 0) return asked.reachesTo === undefined ? "Permission" : `Permission to ${asked.reachesTo}`;
  const header = (asked?.questions ?? lapsed?.questions)?.[0]?.header ?? "";
  return header === "" ? "Question" : header;
}

/** The question's own words, or a permission's command: the line its row in the Needs you list reads. */
export function askText({ asked, lapsed }: Pick<SessionThreadProps, "asked" | "lapsed">): string {
  if (asked !== undefined) return asked.questions?.[0]?.question ?? asked.command.split("\n")[0] ?? "";
  return lapsed?.questions[0]?.question ?? "";
}

/** What the agent is held on, or asked and was not answered: the permission or question card, then the lapsed deck when no call holds it. Answered in place, wherever it is drawn. */
export function AskCards({ asked, askRef, onAnswer, lapsed, flat }: Pick<SessionThreadProps, "asked" | "askRef" | "onAnswer" | "lapsed"> & { /** Inside a panel that frames it: the question deck without its own card, band or fold. */ flat?: true }) {
  return (
    <>
      {asked === undefined ? null : (
        <Card flat ref={askRef} className="armada-session-thread__ask" role="article" aria-label="Waiting on you">
          {(asked.questions?.length ?? 0) > 0 ? null : (
            <span className="armada-session-thread__eyebrow">{asked.reachesTo === undefined ? "Permission" : `Permission to ${asked.reachesTo}`}</span>
          )}
          {(asked.questions?.length ?? 0) > 0 ? null : allowsOnce(asked) ? (
            // The command is a block to be run, as in the thread: ▶ is Allow once.
            <div className="armada-prose__run" role="group" aria-label={asked.command.split("\n")[0]}>
              <div className="armada-prose__run-block">
                <pre className="armada-prose__block">
                  <code>{asked.command}</code>
                </pre>
                <span className="armada-prose__run-acts">
                  <Tooltip label="Allow once">
                    <Button variant="ghost" size="sm" iconOnly aria-label="Allow once" onClick={() => onAnswer("allow_once")}>
                      <Play size={16} strokeWidth={2} aria-hidden />
                    </Button>
                  </Tooltip>
                </span>
              </div>
            </div>
          ) : (
            <p className="armada-session-thread__command">{asked.command}</p>
          )}
          {asked.questions !== undefined && asked.questions.length > 0 ? (
            <QuestionDeck
              key={asked.command}
              {...(flat === undefined ? {} : { flat })}
              questions={asked.questions}
              onAnswer={onAnswer}
              canSkip={asked.offers === undefined || asked.offers.some((offer) => offer.id === "refuse")}
            />
          ) : asked.reaches !== undefined && asked.offers?.some((offer) => FOLDER_ANSWERS.has(offer.id)) === true ? (
            <ReachAnswers key={asked.reaches} reaches={asked.reaches} {...(asked.reachesTo === undefined ? {} : { reachesTo: asked.reachesTo })} offers={asked.offers} onAnswer={onAnswer} />
          ) : (
          <div className="armada-session-thread__answers" role="group" aria-label="Answers">
            {asked.offers === undefined ? (
              <>
                <Button size="sm" variant="secondary" onClick={() => onAnswer()}>
                  Allow once
                </Button>
                <Button size="sm" variant="secondary" onClick={() => onAnswer()}>
                  Deny
                </Button>
              </>
            ) : (
              asked.offers.filter((offer) => offer.id !== "allow_once").map((offer) => (
                <Tooltip key={offer.id} label={offer.means}>
                  <Button size="sm" variant="secondary" onClick={() => onAnswer(offer.id)}>
                    {offer.label}
                  </Button>
                </Tooltip>
              ))
            )}
          </div>
          )}
        </Card>
      )}
      {lapsed === undefined || asked !== undefined ? null : (
        <Card flat className="armada-session-thread__ask" role="article" aria-label="Unanswered">
          <QuestionDeck
            key={lapsed.id}
            {...(flat === undefined ? {} : { flat })}
            questions={lapsed.questions}
            onAnswer={(answer, answers) => {
              if (answers !== undefined) lapsed.onAnswer(answers);
              else if (answer === "refuse") lapsed.onSkip();
            }}
            canSkip
          />
        </Card>
      )}
    </>
  );
}

export function SessionThread({ rows, onRetry, approval, asked, askRef, onAnswer, lapsed, workingOn, onOpenSession, onOpenWindow, onOpenSketch, onOpenCheck, onOpenRetro, onOpenSent, sessionId, working, onStop, agentName, runner, onCommand, commandNames, onForkAt, onReplyAt, lead, transcript }: SessionThreadProps) {
  const whole = useRef<HTMLDivElement>(null);
  useCopyOnSelect(whole);
  const slash = useMemo(() => (onCommand === undefined ? null : { send: onCommand, names: new Set(commandNames ?? []) }), [onCommand, commandNames]);
  const scroller = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  // **Opens at the newest row, with no animation, and stays there while the person is at the end.**
  // A person who scrolled up to read is not moved by a row that arrives.
  useLayoutEffect(() => {
    pinned.current = true;
  }, [sessionId]);
  useLayoutEffect(() => {
    const one = scroller.current;
    if (one !== null && pinned.current) one.scrollTop = one.scrollHeight;
  }, [rows, sessionId, working, approval === undefined ? 0 : approval.approved ? 2 : 1, asked?.command ?? ""]);
  const all = useMemo(() => itemsOf(rows), [rows]);
  const [drawn, setDrawn] = useState(DRAWN);
  useLayoutEffect(() => {
    setDrawn(DRAWN);
  }, [sessionId]);
  const hidden = Math.max(0, all.length - drawn);
  const items = hidden === 0 ? all : all.slice(hidden);
  const endsInCalls = items[items.length - 1]?.kind === "calls";
  const asking = (approval !== undefined && !approval.approved) || asked !== undefined || lapsed !== undefined;
  // What waits on him is the thread's last card, answered in place.
  const cards = (
    <>
      {approval === undefined || approval.approved ? null : <PlanCard approval={approval} />}
      <AskCards {...(asked === undefined ? {} : { asked })} {...(askRef === undefined ? {} : { askRef })} onAnswer={onAnswer} {...(lapsed === undefined ? {} : { lapsed })} />
    </>
  );
  return (
    <div className="armada-session-thread" ref={whole}>
      {onForkAt === undefined && onReplyAt === undefined ? null : <ForkFromSelection scope={whole} {...(onForkAt === undefined ? {} : { onFork: onForkAt })} {...(onReplyAt === undefined ? {} : { onReply: onReplyAt })} />}
      <div className="armada-session-thread__stage">
      <div
        ref={scroller}
        className="armada-session-thread__rows"
        role="region"
        aria-label="Thread"
        tabIndex={0}
        onKeyDown={(event) => {
          // The thread scrolls from the keyboard when it has the focus, smoothly unless motion is reduced, so the pinned region is seen to follow.
          const one = event.currentTarget;
          if (event.target !== one || event.metaKey || event.ctrlKey || event.altKey) return;
          const by = event.key === "PageUp" ? -one.clientHeight * 0.9 : event.key === "PageDown" ? one.clientHeight * 0.9 : event.key === "Home" ? -one.scrollTop : event.key === "End" ? one.scrollHeight : 0;
          if (by === 0) return;
          event.preventDefault();
          one.scrollBy({ top: by, behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
        }}
        onScroll={(event) => {
          const one = event.currentTarget;
          pinned.current = one.scrollHeight - one.scrollTop - one.clientHeight <= NEAR_END;
        }}
      >
        <ProseWalk.Provider value={onOpenWindow ?? null}>
        <ProseRunner.Provider value={runner ?? null}>
        <ProseCommand.Provider value={slash}>
        <ol className="armada-helm-thread__rows" {...(transcript === "unread" ? { "aria-busy": true } : {})}>
          {lead === undefined ? null : <li className="armada-session-thread__lead">{lead}</li>}
          {transcript === "unread" ? <TranscriptSkeleton /> : null}
          {hidden === 0 ? null : (
            <li className="armada-session-thread__earlier">
              <Tooltip label="Show earlier">
                <Button variant="ghost" size="sm" aria-label="Show earlier" onClick={() => setDrawn((now) => now + DRAWN)}>
                  <ChevronUp size={12} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            </li>
          )}
          {items.flatMap((item, at) => [
            item.kind === "calls" ? (
              <Calls key={item.id} rows={item.rows} working={working === true && at === items.length - 1} {...(onStop === undefined ? {} : { onStop })} />
            ) : item.kind === "checks" ? (
              <CheckRuns key={item.id} rows={item.rows} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} />
            ) : (
              <Row key={"stableKey" in item.row && item.row.stableKey !== undefined ? item.row.stableKey : item.row.id} row={item.row} {...(onRetry === undefined ? {} : { onRetry })} onOpenSession={onOpenSession} onOpenWindow={onOpenWindow} onOpenSketch={onOpenSketch} onOpenRetro={onOpenRetro} onOpenSent={onOpenSent} forkable={onForkAt !== undefined || onReplyAt !== undefined} {...(agentName === undefined ? {} : { agentName })} />
            ),
            approval?.approved === true && item.kind === "row" && item.row.id === approval.after ? <li key="plan-approved" className="armada-session-thread__record"><PlanCard approval={approval} /></li> : null,
          ])}
          {transcript === "refreshing" ? (
            <li className="armada-session-thread__refreshing">
              <Tooltip label="Loading new messages">
                <span className="armada-session-thread__loading" role="img" aria-label="Loading new messages">
                  <LoaderCircle size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            </li>
          ) : null}
          {asking ? (
            <li className="armada-session-thread__tail" aria-label="Waiting on you">
              {cards}
            </li>
          ) : null}
          {working === true && !endsInCalls ? (
            <li className="armada-session-thread__working">
              <SessionMark state="working" said="Working" />
              {onStop === undefined ? null : <SessionStop onStop={onStop} />}
            </li>
          ) : null}
        </ol>
        </ProseCommand.Provider>
        </ProseRunner.Provider>
        </ProseWalk.Provider>
      </div>
      </div>
      <div className="armada-session-thread__footer">
      {workingOn === undefined || workingOn.todos.length === 0 ? null : <WorkingOn todos={workingOn.todos} />}
      </div>
    </div>
  );
}

const TASK_OF: Record<WorkingTodo["state"], TaskMarkState> = { done: "done", doing: "working", todo: "open" };

/** A todo's state as `TaskMark` draws a task's, named with its words on hover. Only the one being done breathes. */
function TodoMark({ todo, tone }: { todo: WorkingTodo; tone?: true }) {
  const task = TASK_OF[todo.state];
  const Glyph = TASK_GLYPH[task];
  const said = `${taskMarkSaid(task)}: ${todo.text}`;
  return (
    <Tooltip label={said}>
      <span className={tone === undefined ? "armada-session-mark" : "armada-session-mark armada-task-mark"} data-state={task} data-pulsing={todo.state === "doing" || undefined} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** Folded, the band shows a glyph per todo, named on hover; a press on the band opens the rows. Nothing is drawn for a list that is empty. */
function WorkingOn({ todos }: { todos: readonly WorkingTodo[] }) {
  const [open, setOpen] = useState(false);
  const Fold = open ? ChevronDown : ChevronUp;
  return (
    <section className="armada-session-working" aria-label="Working on">
      <button type="button" className="armada-session-working__band" aria-expanded={open} onClick={() => setOpen(!open)}>
        <span className="armada-session-working__label">Working on</span>
        {open ? null : (
          <span className="armada-session-working__glyphs">
            {todos.map((todo) => (
              <TodoMark key={todo.key} todo={todo} />
            ))}
          </span>
        )}
        <Fold className="armada-session-working__fold" size={12} strokeWidth={2} aria-hidden />
      </button>
      {open ? (
        <ul className="armada-session-working__rows">
          {todos.map((todo) => (
            <li key={todo.key} className="armada-session-working__row" data-state={todo.state}>
              <TodoMark todo={todo} tone />
              <span className="armada-session-working__text">{todo.text}</span>
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
