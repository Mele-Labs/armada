import { useContext, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent as ReactMouseEvent, ReactNode, Ref, RefObject } from "react";
import { AppWindow, BadgeCheck, BellRing, Bot, Box, Check, ChevronDown, ChevronRight, ChevronUp, GitBranch, GitPullRequest, KeyRound, Layers, Play, Reply, Server, Smartphone, SquareTerminal, Wrench } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Prose, ProseCommand, ProseRefs, ProseRunner, ProseWalk } from "../../primitives/Prose/Prose";
import type { CodeRunner } from "../../primitives/Prose/Prose";
import { TerminalMark } from "../../primitives/TerminalPane/TerminalPane";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ApprovalCard } from "../HelmThread/ApprovalCard";
import type { HelmApprovalCard } from "../HelmThread/ApprovalCard";
import { SentTo } from "../HelmThread/SentTo";
import type { ThoughtTarget } from "../ThoughtCard/ThoughtCard";
import { SessionMark } from "../SessionFrame/SessionFrame";
import { SessionStop } from "./SessionStop";
import { TASK_GLYPH, TaskMark, taskMarkSaid, type TaskMarkState } from "../TaskMark/TaskMark";
import { ARTIFACT, LedgerRows, PullRequestChecksMark, PullRequestStateMark, waitingMark, type LedgerEntry } from "../SessionLedger/SessionLedger";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";
import { InlineTag, titleWithoutReference } from "../SessionComposer/InlineTag";
import { FORK_ROW, ForkFromSelection } from "../ForkFromSelection/ForkFromSelection";
import type { ForkPick } from "../ForkFromSelection/ForkFromSelection";
import { SessionRetroRow, type SessionRetroRowData } from "../SessionRetroRow/SessionRetroRow";
import { QuestionDeck } from "./QuestionDeck";
import { MovedRow } from "./MovedRow";
import { StartedRow } from "./StartedRow";
import { SketchRow } from "./SketchRow";
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
    }
  | { id: string; at: string; kind: "message"; from: "session"; sender: { id: string; title: string }; text: string }
  /** Words typed in Helm's box that went to a Session or a Job and never to Helm: Helm's own thread has these, a Session's does not. */
  | {
      id: string;
      at: string;
      kind: "sent";
      text: string;
      target: ThoughtTarget & { id: string };
      files?: readonly { id: string; name: string; src?: string }[];
      sketches?: readonly { id: string; title: string }[];
      tags?: readonly ComposerTag[];
    }
  /** Helm's `ask_person_to_approve` call, drawn as the card that asks for the press on that Job's gate. */
  | { id: string; at: string; kind: "approval"; card: HelmApprovalCard }
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
  /** The Session was moved to another repository, or is the one that was: a quiet row naming where, a press that opens the other. */
  | { id: string; at: string; kind: "moved"; way: "to" | "from"; session: string; repository: string }
  /** A retro being written for this Session, then the one that was: a card that stays in the thread. */
  | SessionRetroRowData
  | {
      id: string;
      at: string;
      kind: "handoff";
      job: { number: number; title: string };
      slot: number;
      branch: string;
      /** Absent where the Job stopped on no step. */
      step?: { id: string; label: string };
      attempts: number;
      refusals: readonly string[];
      /** Files written that the plan did not cover, and paths it named that nothing changed under. */
      plan: { outside: readonly string[]; unwritten: readonly string[] };
      /** Absent where no Drone said what it was stuck on: a Job taken over from an escalation or a gate has none. */
      narrative?: { trying_to: string; blocked_by: string; tried: readonly string[] };
    };

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
  groups: readonly { id: string; agent: string; title: string; tasks: readonly { id: string; text: string }[] }[];
  approved: boolean;
  after?: string;
  /** A sketch drawn with the plan, shown under the groups as a sketch card is and opened as one is. */
  sketch?: { id: string; title: string; scene: Scene };
  onApprove: () => void;
  /** Opens the plan to read in full: a press on the card anywhere but Approve, and on its sketch. Absent leaves the card plain. */
  onReview?: () => void;
};

export type SessionThreadProps = {
  rows: readonly SessionThreadRow[];
  /** A plan waiting on the person, or settled. Absent draws none. */
  approval?: PlanApproval;
  /**
   * The permission the agent is held on. **`offers` are the answers Fleet will take**, drawn in its
   * order; absent, the card offers Allow once and Deny and names neither.
   */
  asked?: { command: string; offers?: readonly AskOffer[]; questions?: readonly AskedQuestion[] };
  onAnswer: (answer?: string, answers?: QuestionAnswer[]) => void;
  /**
   * A question the agent asked that nobody answered in time, or that was skipped, drawn as the deck under the
   * thread until it is answered or the person says anything else. Not a call: the agent is not held on it, so
   * Skip only puts the deck away, and a live `asked` takes its place.
   */
  lapsed?: { id: string; questions: readonly AskedQuestion[]; onAnswer: (answers: QuestionAnswer[]) => void; onSkip: () => void };
  /**
   * The agent says it has nothing left: a callout ends the thread, with the two ways on. Absent draws none.
   * `made` are ledger entries (pull requests, or artifacts), listed under the band as presses; none draws no body.
   * A pull request's entry carries `pr`: its state and Checks, each a bare mark with a tooltip.
   */
  done?: { onEnd: () => void; onKeep: () => void; made?: readonly LedgerEntry[] };
  /**
   * What the Session waits on the person for, in the done callout's place while it has not said it is done:
   * a band of one glyph per item, which a press opens to the ledger's own rows. Empty or absent draws none.
   */
  waiting?: readonly LedgerEntry[];
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
  /** Opens the Session or Job a row of Helm's sent words went to. Absent, that row's chip is not a press. */
  onOpenSent?: (target: { kind: "session" | "job"; id: string }) => void;
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
};

const TAG_KIND = { session: "Session", job: "Job", pull_request: "Pull request", branch: "Branch", issue: "Issue" } as const;

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

/** What Fleet knew when a Job's Drone stopped, as the structured fields the handoff bundle has. A field with nothing to say is left out. */
function Handoff({ row }: { row: Extract<SessionThreadRow, { kind: "handoff" }> }) {
  const { step, narrative } = row;
  const planned = row.plan.outside.length + row.plan.unwritten.length > 0;
  return (
    <li className="armada-session-handoff" role="region" aria-label={`Handed over: Job ${row.job.number}`}>
      <div className="armada-session-handoff__head">
        <span className="armada-session-lease__eyebrow">Handed over</span>
        <span className="armada-session-lease__facts">
          <span className="armada-session-lease__chip" role="img" aria-label={`Job ${row.job.number}`}>
            <Box size={12} strokeWidth={2} aria-hidden />
            {row.job.number}
          </span>
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
      </div>
      <dl className="armada-session-handoff__fields">
        {step === undefined ? null : (
          <>
            <dt>Stopped on</dt>
            <dd>
              <code>{step.id}</code> {step.label}
            </dd>
            <dt>Attempts</dt>
            <dd>
              <code>{row.attempts}</code>
            </dd>
          </>
        )}
        {row.refusals.length === 0 ? null : (
          <>
            <dt>Judge refused</dt>
            <dd>
              <ul>
                {row.refusals.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ul>
            </dd>
          </>
        )}
        {!planned ? null : (
          <>
            <dt>Plan against diff</dt>
            <dd>
              <ul>
                {row.plan.outside.map((one) => (
                  <li key={one}>
                    <code>{one}</code> written, not declared
                  </li>
                ))}
                {row.plan.unwritten.map((one) => (
                  <li key={one}>
                    <code>{one}</code> declared, not written
                  </li>
                ))}
              </ul>
            </dd>
          </>
        )}
        {narrative === undefined ? null : (
          <>
            <dt>Trying to</dt>
            <dd>{narrative.trying_to}</dd>
            <dt>Blocked by</dt>
            <dd>{narrative.blocked_by}</dd>
            <dt>Tried</dt>
            <dd>
              <ol>
                {narrative.tried.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ol>
            </dd>
          </>
        )}
      </dl>
    </li>
  );
}

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

function actOf(text: string): Act | null {
  const approved = /^Approved: (\S+)$/.exec(text);
  if (approved?.[1] !== undefined) {
    const walk = /[?&]walk=[^&#\s]*/.exec(approved[1]);
    return { kind: "approved", target: approved[1], shown: walk === null ? approved[1] : `?${walk[0].slice(1)}` };
  }
  const answered = /^Re: ([^\n]+)\n?([\s\S]*)$/.exec(text);
  if (answered?.[1] !== undefined) return { kind: "answered", target: answered[1], said: (answered[2] ?? "").trim() };
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

function Row({
  row,
  onOpenSession,
  onOpenWindow,
  onOpenSketch,
  agentName = "Agent",
  onOpenRetro,
  onOpenSent,
  forkable,
}: {
  row: Exclude<SessionThreadRow, { kind: "tool" | "check" }>;
  forkable: boolean;
  onOpenSession: (id: string) => void;
  onOpenWindow?: (url: string) => void;
  onOpenSketch?: (sketch: string) => void;
  agentName?: string;
  onOpenRetro?: (retro: number) => void;
  onOpenSent?: (target: { kind: "session" | "job"; id: string }) => void;
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
          <pre className="armada-session-fold__tail" role="log" aria-label="Sent to the Session">
            {row.tail}
          </pre>
        </details>
      </li>
    );
  }
  if (row.kind === "sketch") return <SketchRow row={row} {...(onOpenSketch === undefined ? {} : { onOpen: onOpenSketch })} />;
  if (row.kind === "approval") {
    return (
      <li role="region" aria-label="Approval asked for">
        <ApprovalCard {...row.card} />
      </li>
    );
  }
  if (row.kind === "handoff") return <Handoff row={row} />;
  if (row.kind === "started") return <StartedRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "moved") return <MovedRow row={row} onOpen={onOpenSession} />;
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
        <button type="button" className="armada-session-from__band" aria-label={`Open Session ${sender.title}`} onClick={() => onOpenSession(sender.id)}>
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
    const placed = placeTags(row.text, row.tags ?? [], openTag);
    return (
      <li className="armada-session-you" role="region" aria-label="Your message">
        <div className="armada-session-you__band">
          <span className="armada-session-you__who">You</span>
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
        </div>
      </li>
    );
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

/** How far from the end still counts as being at it, so a rounding or a half row does not unpin. */
const NEAR_END = 24;

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

/** A permission that offers Allow once draws its command as a block to run, and ▶ is that offer. */
/**
 * The plan as a compact card: each group a closed row (its subagent type, its title and a mark for each task), a small thumbnail of
 * the sketch drawn with it, and Approve. **A press anywhere on it but Approve opens the plan in full** (`onReview`), where every group is
 * open and the sketch is large. Short enough that the thread never has to scroll it under the box.
 */
function PlanCard({ approval }: { approval: PlanApproval }) {
  const { approved, onReview } = approval;
  const review = onReview === undefined ? undefined : { tabIndex: 0, "data-reviewable": "", onClick: (event: ReactMouseEvent<HTMLElement>) => { if ((event.target as Element).closest("button") === null) onReview(); }, onKeyDown: (event: KeyboardEvent<HTMLElement>) => { if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); onReview(); } } };
  return (
    <Card flat className="armada-session-thread__ask armada-session-thread__plan" role="article" aria-label={approved ? "Plan approved" : "Plan to approve"} data-approved={approved || undefined} {...review}>
      <span className="armada-session-thread__eyebrow">
        Plan
        {approved ? (
          <Tooltip label="Plan approved">
            <span className="armada-session-thread__settled" role="img" aria-label="Plan approved">
              <BadgeCheck size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        ) : null}
      </span>
      <div className="armada-session-thread__plan-body">
        <ul className="armada-session-thread__groups">
          {approval.groups.map((group) => (
            <li key={group.id} className="armada-session-thread__group">
              <Tooltip label="Subagent type">
                <span className="armada-session-thread__agent" role="img" aria-label={`Subagent type ${group.agent}`}>
                  <Bot size={12} strokeWidth={2} aria-hidden />
                  <code>{group.agent}</code>
                </span>
              </Tooltip>
              <span className="armada-session-thread__group-title">{group.title}</span>
              {approved ? null : (
                <span className="armada-session-thread__marks">
                  {group.tasks.map((task) => (
                    <TaskMark key={task.id} state="open" />
                  ))}
                </span>
              )}
            </li>
          ))}
        </ul>
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
    </Card>
  );
}

const allowsOnce = (asked: NonNullable<SessionThreadProps["asked"]>) => asked.offers?.some((offer) => offer.id === "allow_once") === true;

export function SessionThread({ rows, approval, asked, askRef, onAnswer, lapsed, done, waiting, workingOn, onOpenSession, onOpenWindow, onOpenSketch, onOpenCheck, onOpenRetro, onOpenSent, sessionId, working, onStop, agentName, runner, onCommand, commandNames, onForkAt }: SessionThreadProps) {
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
  }, [rows, sessionId, working, approval === undefined ? 0 : approval.approved ? 2 : 1]);
  const items = itemsOf(rows);
  const endsInCalls = items[items.length - 1]?.kind === "calls";
  return (
    <div className="armada-session-thread" ref={whole}>
      {onForkAt === undefined ? null : <ForkFromSelection scope={whole} onFork={onForkAt} />}
      <div
        ref={scroller}
        className="armada-session-thread__rows"
        role="region"
        aria-label="Thread"
        onScroll={(event) => {
          const one = event.currentTarget;
          pinned.current = one.scrollHeight - one.scrollTop - one.clientHeight <= NEAR_END;
        }}
      >
        <ProseWalk.Provider value={onOpenWindow ?? null}>
        <ProseRunner.Provider value={runner ?? null}>
        <ProseCommand.Provider value={slash}>
        <ol className="armada-helm-thread__rows">
          {items.flatMap((item, at) => [
            item.kind === "calls" ? (
              <Calls key={item.id} rows={item.rows} working={working === true && at === items.length - 1} {...(onStop === undefined ? {} : { onStop })} />
            ) : item.kind === "checks" ? (
              <CheckRuns key={item.id} rows={item.rows} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} />
            ) : (
              <Row key={item.row.id} row={item.row} onOpenSession={onOpenSession} onOpenWindow={onOpenWindow} onOpenSketch={onOpenSketch} onOpenRetro={onOpenRetro} onOpenSent={onOpenSent} forkable={onForkAt !== undefined} {...(agentName === undefined ? {} : { agentName })} />
            ),
            approval?.approved === true && item.kind === "row" && item.row.id === approval.after ? <li key="plan-approved" className="armada-session-thread__record"><PlanCard approval={approval} /></li> : null,
          ])}
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
      {approval === undefined || approval.approved ? null : <PlanCard approval={approval} />}
      {asked === undefined ? null : (
        <Card flat ref={askRef} className="armada-session-thread__ask" role="article" aria-label="Waiting on you">
          {(asked.questions?.length ?? 0) > 0 ? null : <span className="armada-session-thread__eyebrow">Permission</span>}
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
              questions={asked.questions}
              onAnswer={onAnswer}
              canSkip={asked.offers === undefined || asked.offers.some((offer) => offer.id === "refuse")}
            />
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
            questions={lapsed.questions}
            onAnswer={(answer, answers) => {
              if (answers !== undefined) lapsed.onAnswer(answers);
              else if (answer === "refuse") lapsed.onSkip();
            }}
            canSkip
          />
        </Card>
      )}
      {done !== undefined || workingOn === undefined || workingOn.todos.length === 0 ? null : <WorkingOn todos={workingOn.todos} />}
      {done !== undefined || waiting === undefined || waiting.length === 0 ? null : <NeedsYou rows={waiting} />}
      {done === undefined ? null : (
        <div className="armada-session-done" role="article" aria-label="Nothing left to do">
          <div className="armada-session-done__head">
            <Tooltip label="The agent has nothing left to work on">
              <span className="armada-session-done__mark" role="img" aria-label="Nothing left to work on">
                <Check size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            <span className="armada-session-done__label">Nothing left to do</span>
          </div>
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
              End session
            </Button>
            <Button size="sm" variant="ghost" onClick={done.onKeep}>
              Keep going
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

/** Folded, the band shows a glyph per item, named on hover; a press on the band opens the rows. */
function NeedsYou({ rows }: { rows: readonly LedgerEntry[] }) {
  const [open, setOpen] = useState(false);
  const Fold = open ? ChevronDown : ChevronUp;
  return (
    <section className="armada-session-needs" aria-label="Needs you">
      <button type="button" className="armada-session-needs__band" aria-expanded={open} onClick={() => setOpen(!open)}>
        <BellRing size={12} strokeWidth={2} aria-hidden />
        <span className="armada-session-needs__label">Needs you</span>
        {open ? null : (
          <span className="armada-session-needs__glyphs">
            {rows.map((row) => {
              const { Glyph } = waitingMark(row);
              return (
                <Tooltip key={row.key} label={row.name}>
                  <span className="armada-session-mark" role="img" aria-label={row.name}>
                    <Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              );
            })}
          </span>
        )}
        <Fold className="armada-session-needs__fold" size={12} strokeWidth={2} aria-hidden />
      </button>
      {open ? (
        <div className="armada-session-needs__rows">
          <LedgerRows kind="waiting" Glyph={BellRing} rows={rows} />
        </div>
      ) : null}
    </section>
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
