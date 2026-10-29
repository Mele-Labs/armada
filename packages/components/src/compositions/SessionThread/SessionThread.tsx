import { useContext, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent as ReactMouseEvent, ReactNode, Ref, RefObject } from "react";
import { AppWindow, ArrowRight, BadgeCheck, BellRing, Bot, Check, ChevronDown, ChevronRight, ChevronUp, Clock, File, GitBranch, GitPullRequest, KeyRound, Layers, ListChecks, LoaderCircle, MessageSquare, MoveRight, Pencil, Play, Plus, Reply, Scale, Server, Smartphone, SquareTerminal, Trash2, Wrench, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { CallKind, DraftSettled, ToolSubagent } from "@armada/protocol";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Enlargeable } from "../../primitives/Enlargeable/Enlargeable";
import { Card } from "../../primitives/Card/Card";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { PathPicker } from "../../primitives/PathPicker/PathPicker";
import { Skeleton, SkeletonText } from "../../primitives/Skeleton/Skeleton";
import { Switch } from "../../primitives/Switch/Switch";
import { Prose, ProseCommand, ProseRefs, ProseRunner, ProseWalk } from "../../primitives/Prose/Prose";
import type { CodeRunner } from "../../primitives/Prose/Prose";
import { TerminalMark } from "../../primitives/TerminalPane/TerminalPane";
import { Stamp, StampSurface } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SentTo } from "../HelmThread/SentTo";
import type { ThoughtTarget } from "../ThoughtCard/ThoughtCard";
import { SessionMark } from "../SessionFrame/SessionFrame";
import { notedOf } from "../ViewerNotes/compose";
import { NotedMessage, NotedOpens, type NotedOpen } from "../ViewerNotes/NotedMessage";
import { splitDebugInfo } from "./debug-info";
import { SessionStop } from "./SessionStop";
import { TASK_GLYPH, TaskMark, taskMarkSaid, type TaskMarkState } from "../TaskMark/TaskMark";
import { artifactOf, PullRequestChecksMark, PullRequestStateMark, type LedgerEntry } from "../SessionLedger/SessionLedger";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";
import { InlineTag, titleWithoutReference } from "../SessionComposer/InlineTag";
import { FORK_ROW, ForkFromSelection } from "../ForkFromSelection/ForkFromSelection";
import type { ForkPick } from "../ForkFromSelection/ForkFromSelection";
import { SessionRetroRow, type SessionRetroRowData } from "../SessionRetroRow/SessionRetroRow";
import { QuestionDeck } from "./QuestionDeck";
import { ConcernClaim } from "./ConcernClaim";
import { AdversarialReviewPress, ReviewFailed, findingsOf } from "../AdversarialReview/AdversarialReview";
import { FindingNotes, GivesRow, GroupFindings, anchorsOf, useFindingKeys } from "../AdversarialReview/AnchoredFindings";
import { isPressed } from "../../keymap";
import type { AdversarialReviewProps } from "../AdversarialReview/AdversarialReview";
import { LinkedRow } from "./LinkedRow";
import { DisputedMark, DisputedRow } from "./DisputedRow";
import { FleetTurn, noticeEntries } from "./FleetTurn";
import type { TurnReply } from "./FleetTurn";
import { useTeachMenu } from "../TeachPanel/TeachMenu";
import { MovedRow } from "./MovedRow";
import { StartedRow } from "./StartedRow";
import { SubagentRow } from "./SubagentRow";
import { WaiterRow } from "./WaiterRow";
import { SketchRow } from "./SketchRow";
import type { SketchAnswer, SketchSettled } from "./sketch-answer";
import { RequestedRow } from "./RequestedRow";
import { SavedRow } from "./SavedRow";
import { DraftRow } from "./DraftRow";
import { ThreadKbd, useThreadCard } from "./thread-keys";
import type { DraftAnswer } from "./DraftRow";
export type { DraftAnswer } from "./DraftRow";
import { callsLine } from "./calls-line";
import { PlanRevisionView, useUnchanged } from "./PlanRevision";
import type { PlanRevision } from "./PlanRevision";
export type { PlanRevision, RevisionGroup, RevisionSince, RevisionTask } from "./PlanRevision";
export { PlanRevisionView, useUnchanged } from "./PlanRevision";
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
/** A Fleet notice a person can turn off, and the reach of one choice: this Job, its repository, or this Job for an hour. */
export type SessionNotice =
  | "behind_base"
  | "pr_checks_failed"
  | "pr_conflicts"
  | "pr_dropped"
  | "pr_unmergeable"
  | "pr_merged"
  | "pr_auto_merge"
  | "pr_queued"
  | "pr_handed"
  | "slot_free"
  | "slot_kept"
  | "build_moved"
  | "write_clash"
  | "slot_stranded"
  | "main_red"
  | "rail_nudge"
  | "rail_step"
  | "usage_trip"
  | "limit_wake"
  | "sign_in_wake"
  | "plan_approved"
  | "auto_land_refused"
  | "retro_note"
  | "reply_claim"
  | "context_handoff"
  | "walk_failed"
  | "workflow_changed"
  | "delivery_asked"
  | "delivery_chosen"
  | "watch_saw"
  | "taught"
  | "issue_lost"
  | "document_sketch"
  | "task_marked";
export type NoticeChoice = "job" | "repo" | "hour";

/** What Teach was pressed on in a row: a tool call, a Fleet notice, or a Check. `at` is when the row was written. */
export type TeachPick = { kind: "command" | "notice" | "check"; text: string; at?: string; /** A notice's word, which Fleet reads the rule for. */ ids?: string[] };

type SessionThreadKinds =
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
      /** The person's message, held until the call the agent's turn waits on returns: a quiet mark in its band. */
      queued?: true;
      /** The key the row keeps while Fleet's own row takes its place, so the message does not draw again. */
      stableKey?: string;
    }
  | {
      id: string;
      at: string;
      kind: "message";
      from: "session";
      sender: { id: string; title: string };
      text: string;
      /** The Fleet notice this message was sent for. Where `onMuteNotice` is given, a right press on it offers to turn it off. */
      notice?: SessionNotice;
      /** Several notices Fleet held and sent as one: what the person taught Fleet to batch, named. Each line of `text` is one notice. */
      batch?: string;
    }
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
  /** A tool call, mono. `fromPhone` is an ask the person answered on their phone: a glyph beside it. A call with `subagent` is a row of its own (`SubagentRow`). */
  | { id: string; at: string; kind: "tool"; text: string; fromPhone?: true; subagent?: ToolSubagent }
  /** Fleet woke the agent, and why: a quiet row where the turn it started begins. */
  | { id: string; at: string; kind: "woke"; why: string }
  /** What the agent did on its own, mono, with the mark and the tooltip that name which act it was. */
  | { id: string; at: string; kind: "act"; icon: LucideIcon; said: string; text: string }
  /** A command the person ran in the terminal, as typed, mono. */
  | { id: string; at: string; kind: "command"; text: string }
  /** The summary the CLI wrote where it compacted the conversation. Not the person's words: a quiet row that opens to its text. */
  | { id: string; at: string; kind: "compaction"; text: string }
  /**
   * The agent disputed Fleet's notice, the message row `notice`: one line in Fleet's voice with the reason, how it was
   * spotted, and a press that opens the review Job `review`. Fleet's card for the notice draws the Disputed mark.
   */
  | { id: string; at: string; kind: "disputed"; notice: string; why: string; spotted: "called" | "read"; review?: string }
  | { id: string; at: string; kind: "lease"; slot: number; branch: string }
  | { id: string; at: string; kind: "release"; slot: number; branch: string }
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
  /**
   * An HTTP request sent through Fleet, by the person from Requests or by the agent: who, the mark of how it was answered, the
   * method, the address and the time, and a fold with the answer's body as plain text. The request is whole, to open again.
   * `status` is null where nothing answered and `error` says why. A secret is drawn as its `{{name}}`, never its value.
   */
  | {
      id: string;
      at: string;
      kind: "requested";
      by: "owner" | "agent";
      method: string;
      url: string;
      environment?: string;
      headers: readonly { name: string; value: string }[];
      body?: string;
      status: number | null;
      ms: number;
      error?: string;
      response: string;
      truncated: boolean;
    }
  /**
   * **Proposed, not on the wire**: the agent saved a request into a collection in Requests. `collectionId` and `request` name
   * it there, for the press that opens it.
   */
  | { id: string; at: string; kind: "saved"; name: string; collection: string; collectionId: string; request: string }
  /** The Session showed a page in a window: a quiet row, a press that opens the window again. */
  | { id: string; at: string; kind: "window"; title: string; url: string }
  /** The agent showed a file with `show_file`: a quiet row, a press that opens it in the side panel. */
  | { id: string; at: string; kind: "file"; path: string; name: string }
  /** A draft the agent put to the person with `propose_draft`: a card answered in place, and how it ended once `settled`. */
  | { id: string; at: string; kind: "draft"; title: string; text: string; before?: string; settled?: DraftSettled; /** The workflow change a proposed workflow is. */ change?: number; /** **A mock's field**: a workflow a Job proposed, whose card names it and where it would be kept. */ workflow?: { name: string; scope: "kit" | "repository" }; /** **A mock's field**: a draft that is a rule's own act, whose card is the rule and says where it goes and to whom. */ rule?: { where: string; to: string } }
  /**
   * The agent drew a sketch: a card with a small drawing, a press that opens it. `sketch` is the ledger row's id.
   * `replaced` is set by the host where a later card holds the same `sketch`, and `againstScene` where it finds the card
   * `against` names, so the card draws the change as the panel does. `asked` is a sketch put to the person to approve,
   * and `settled` how he answered it.
   */
  | { id: string; at: string; kind: "sketch"; sketch: string; title: string; drawing: Scene; against?: string; againstScene?: Scene; replaced?: true; asked?: true; settled?: SketchSettled }
  /** This Session's agent started a Session, or forked itself into one: a press that opens it. */
  | { id: string; at: string; kind: "started"; session: { id: string; title: string }; fork: boolean }
  /**
   * A Session that waits on this one, told here: dispatched (`waits`, with the first line of its brief in `says`), its start condition changed (`from` and `to`; `by` is who
   * changed it), or started (`says` is where from). A press opens it.
   */
  | { id: string; at: string; kind: "waiter"; session: { id: string; title: string }; /** Written in the waiting Session's own thread: `session` is the one it waits on. */ waiting?: true; event: "waits" | "changed" | "started"; says?: string; from?: string; to?: string; by?: "you" | "agent" }
  /** A Session this one follows needs the person, finished or merged: the agent was told with it. A press opens that Session. */
  | { id: string; at: string; kind: "linked"; session: { id: string; title: string }; event: "needs" | "done" | "merged" | "started" }
  /** The Session was moved to another repository, or is the one that was: a quiet row naming where, a press that opens the other. */
  | { id: string; at: string; kind: "moved"; way: "to" | "from"; session: string; repository: string }
  /** Where an earlier Job's turns end and this one's begin, after a hand off: a plain rule. */
  | { id: string; at: string; kind: "divider" }
  /** A retro being written for this Session, then the one that was: a card that stays in the thread. */
  | SessionRetroRowData;

/** Every row's `at` is the RFC 3339 instant it happened. A mock's may be words, and the stamp then draws them as they came. */
export type SessionThreadRow = SessionThreadKinds;

/** The row's time as a stamp that flips with the others in the thread. */
function RowStamp({ row, className }: { row: { at: string }; className: string }) {
  return <Stamp at={row.at} className={className} />;
}

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
  /** The plan in his words, read first. Present, the groups fold under Details; absent, the card draws the groups as it always did. */
  summary?: string;
  groups: readonly { id: string; agent: string; title: string; /** What the group gives him, one line; the title stands in where absent. */ gives?: string; paths?: readonly string[]; tasks: readonly { id: string; text: string }[] }[];
  approved: boolean;
  /** He said Not yet: the card is a chip, `Plan set aside`, that unfolds on press, and nothing waits on him for it. */
  setAside?: true;
  /** Approve was pressed and Fleet has not answered: the card draws approved with a going mark in the state's place. */
  going?: true;
  after?: string;
  /** What the agent changed since the plan before this one: what is added, renamed (and what it was) and removed. */
  changes?: readonly { kind: "added" | "renamed" | "removed"; text: string; from?: string }[];
  /** A later version drawn with what changed marked in place and the groups nothing touched closed to a line. Absent, the card draws the plan whole. */
  revision?: PlanRevision;
  /**
   * An adversarial review of this plan. The press stands in the answers row, and the findings are pinned to the plan: a finding about the
   * whole plan under the summary, a group's under its line, a task's under the task, inside the group that holds it. A failed review's
   * words stand between the plan and its answers.
   */
  adversarial?: AdversarialReviewProps;
  /** A sketch drawn with the plan, shown under the groups as a sketch card is and opened as one is. */
  sketch?: { id: string; title: string; scene: Scene };
  onApprove: () => void;
  /** Not yet: sets the plan aside and tells the agent to keep talking. Absent draws no Not yet. */
  onSetAside?: () => void;
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

/** Something the host draws in the line of the thread, where it happened: a mod a Job made, as its card. Without an `at` it comes after the newest row. */
export type ThreadCard = { key: string; at?: string; node: ReactNode };

/** When an item's last row happened, in milliseconds, or `NaN` where it is words. */
const timeOf = (item: Item): number => {
  const at = item.kind === "row" ? item.row.at : item.kind === "subagent" ? item.at : item.kind === "calls" || item.kind === "checks" ? item.rows[item.rows.length - 1]?.at : (item.reply[item.reply.length - 1] ?? item.notice).at;
  return at === undefined ? Number.NaN : Date.parse(at);
};

/** The index of the item a card comes after: the newest one written by its time, the first when it came before them all, the last when it has no time. */
function afterWhich(items: readonly Item[], card: ThreadCard): number {
  const when = card.at === undefined ? Number.NaN : Date.parse(card.at);
  if (Number.isNaN(when)) return items.length - 1;
  let found = 0;
  items.forEach((item, at) => {
    if (timeOf(item) <= when) found = at;
  });
  return found;
}

export type SessionThreadProps = {
  /** The Session is waiting on the person, whose question the host shows elsewhere (the Needs you panel): the thread's last Fleet turn stays open. */
  waitingOnYou?: boolean;
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
   * the call reads that path or writes to it, in the card's eyebrow and the Always head. `kind` is the
   * Bash call's program and script: with an `allow_like_this_for_session` offer the card draws a Calls
   * like row, and that press hands back the kind chosen.
   */
  asked?: { command: string; offers?: readonly AskOffer[]; questions?: readonly AskedQuestion[]; reaches?: string; reachesTo?: "read" | "write"; kind?: CallKind };
  onAnswer: (answer?: string, answers?: QuestionAnswer[], folder?: string, kind?: CallKind) => void;
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
  /** Opens a file the agent showed, by its path. Absent, its row is not a press. */
  onOpenFile?: (path: string) => void;
  /** Answers a draft card, by its row's id. Absent, a draft draws with nothing to press. */
  onDraft?: (rowId: string, answer: DraftAnswer) => void;
  /** Withdraws a draft, by its row's id, with his note as the reason where he typed one. Absent, the card has no Withdraw. */
  onWithdrawDraft?: (rowId: string, reason?: string) => void;
  /** Answers a sketch put to him, by its ledger id. Absent, an asked sketch draws with nothing to press. */
  onAnswerSketch?: (sketch: string, answer: SketchAnswer) => void;
  /**
   * Opens what a note the person sent from a viewer named: the sketch with its parts picked, the file on its lines, the
   * picture with its region drawn. `root` is the Job's checkout, which a file is shown from. Absent, such a message is words.
   */
  noted?: NotedOpen;
  /** Opens the Session a row of Helm's sent words went to. Absent, that row's chip is not a press. */
  onOpenSent?: (target: { kind: "session"; id: string }) => void;
  /** Opens a Check run the agent made on the Checks page. Absent, its row is not a press. */
  onOpenCheck?: (run: number) => void;
  /** Opens a subagent's own thread, by its ledger id. Absent, its row is not a press. */
  onOpenSubagent?: (id: string) => void;
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
  /** Turns a Fleet notice off. Absent, a notice draws as any message and has no menu. */
  onMuteNotice?: (notice: SessionNotice, choice: NoticeChoice) => void;
  /**
   * Teaches Armada about a tool call, a Fleet notice or a Check: a right press on its row, the ContextMenu key or Shift+F10 opens a
   * menu with Teach. Absent where nothing serves it, and those rows have no menu.
   */
  onTeachRow?: (pick: TeachPick) => void;
  onForkAt?: (pick: ForkPick) => void;
  /**
   * Replies to the words the person selected in a message: the bar offers it beside Fork. The caller
   * puts the quote in the message box. Absent where there is no box, and the act is left off.
   */
  onReplyAt?: (pick: ForkPick) => void;
  /**
   * Teaches Armada what the selected words are and what to do with them: the bar offers it beside Reply and Fork, and
   * the menu a right press on the words opens leads with it. Absent where nothing serves it, and the act is left off.
   */
  onTeachAt?: (pick: ForkPick) => void;
  /** Drawn first in the thread, a row like the rest: it scrolls away with them. */
  lead?: ReactNode;
  /**
   * Drawn last, after the newest row and whatever waits on the person: an entry that is still being
   * written, such as a turn held until it is sent. **The thread follows it as it grows**, the way it
   * follows a new row, for a person who is at the end. Absent, nothing is drawn and the thread is as it was.
   */
  trailing?: ReactNode;
  /** Cards drawn inside the thread, each after the newest row written before its `at`. */
  inline?: readonly ThreadCard[];
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
          <Enlargeable key={file.id} className="armada-session-sent__picture" src={file.src} name={file.name} />
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
  | { kind: "fleetTurn"; id: string; notice: Extract<SessionThreadRow, { kind: "message"; from: "session" }>; reply: readonly TurnReply[]; handled: boolean }
  | { kind: "checks"; id: string; rows: readonly CheckRow[] }
  | { kind: "subagent"; id: string; at: string; subagent: ToolSubagent };

/**
 * A turn Fleet started folds into one item: its notice, then the agent's replies and tool runs until the next row of any other
 * kind. The person's message is theirs, not Fleet's turn, so it ends the fold. A notice in `keep` is left as it came.
 */
function itemsOf(rows: readonly SessionThreadRow[], keep: ReadonlySet<string> = new Set()): Item[] {
  const items: Item[] = [];
  for (let at = 0; at < rows.length; at += 1) {
    const row = rows[at]!;
    if (row.kind === "message" && row.from === "session" && row.sender.id === "fleet" && !keep.has(row.id)) {
      const reply: TurnReply[] = [];
      while (at + 1 < rows.length) {
        const next = rows[at + 1]!;
        if (next.kind === "tool" || (next.kind === "message" && next.from === "agent")) reply.push(next);
        else break;
        at += 1;
      }
      // A turn ends in words meant for the person: its last agent message is drawn outside the fold.
      const spoken = reply.map((one) => one.kind === "message").lastIndexOf(true);
      const said = spoken >= 0 ? (reply[spoken] as Extract<TurnReply, { kind: "message" }>) : undefined;
      items.push({ kind: "fleetTurn", id: row.id, notice: row, reply: said === undefined ? reply : reply.filter((one) => one !== said), handled: reply.length > 0 });
      if (said !== undefined) items.push({ kind: "row", row: said });
      continue;
    }
    const last = items[items.length - 1];
    if (row.kind === "check") {
      if (last?.kind === "checks" && last.rows[0]?.name === row.name) last.rows = [...last.rows, row];
      else items.push({ kind: "checks", id: row.id, rows: [row] });
    } else if (row.kind !== "tool") items.push({ kind: "row", row });
    else if (row.subagent !== undefined) items.push({ kind: "subagent", id: row.id, at: row.at, subagent: row.subagent });
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
function Calls({ rows, working, onStop, onTeach }: { rows: readonly { id: string; at: string; text: string; fromPhone?: true }[]; working?: boolean; onStop?: () => void; onTeach?: (pick: TeachPick) => void }) {
  const names = namesOf(rows);
  const line = callsLine(rows, working === true);
  return (
    <li className="armada-session-fold armada-session-fold--calls">
      <details>
        <summary className="armada-session-fold__head" aria-label={`Tool calls: ${names.join(", ")}`}>
          <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-fold__chevron" />
          {working === true ? <SessionMark state="working" said="Working" /> : <Wrench size={12} strokeWidth={2} aria-hidden />}
          {working === true && onStop !== undefined ? <SessionStop onStop={onStop} /> : null}
          <span className="armada-session-fold__names armada-session-fold__names--line">{line}</span>
        </summary>
        <ol className="armada-session-fold__calls">
          {rows.map((one) => (
            <Command key={one.id} row={one} {...(onTeach === undefined ? {} : { onTeach })} />
          ))}
        </ol>
      </details>
    </li>
  );
}

/** One call of a group. Where Teach is offered it takes focus, and the menu opens on it. */
function Command({ row, onTeach }: { row: { id: string; at: string; text: string; fromPhone?: true }; onTeach?: (pick: TeachPick) => void }) {
  const { on, menu } = useTeachMenu("Command", onTeach === undefined ? undefined : () => onTeach({ kind: "command", text: row.text, at: row.at }));
  return (
    <li className="armada-session-command" {...(onTeach === undefined ? {} : { tabIndex: 0 })} {...on}>
      <code className="armada-session-command__text">{row.text}</code>
      {row.fromPhone === true ? (
        <Tooltip label="From phone">
          <span className="armada-session-command__door">
            <Smartphone size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}
      {menu}
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
function CheckRuns({ rows, onOpenCheck, onTeach }: { rows: readonly CheckRow[]; onOpenCheck?: (run: number) => void; onTeach?: (pick: TeachPick) => void }) {
  const latest = rows[rows.length - 1]!;
  const { on, menu } = useTeachMenu("Check", onTeach === undefined ? undefined : () => onTeach({ kind: "check", text: `${latest.name} ${latest.target === undefined ? "" : `${latest.target} `}${latest.state}`.replace(/\s+/g, " ").trim(), at: latest.at }));
  if (rows.length === 1) {
    const inner = (
      <>
        <CheckMark row={latest} />
        <span className="mono">{latest.name}</span>
        {latest.target === undefined ? null : <code className="armada-session-fold__target">{latest.target}</code>}
      </>
    );
    return (
      <li className="armada-session-fold" {...on}>
        {onOpenCheck === undefined ? (
          <span className="armada-session-fold__head" {...(onTeach === undefined ? {} : { tabIndex: 0 })}>{inner}</span>
        ) : (
          <button type="button" className="armada-session-fold__head armada-session-fold__head--press" aria-label={checkSaid(latest)} onClick={() => onOpenCheck(latest.run)}>
            {inner}
          </button>
        )}
        {menu}
      </li>
    );
  }
  return (
    <li className="armada-session-fold armada-session-fold--checks" {...on}>
      {menu}
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
                <Stamp still at={one.at} className="armada-session-fold__at" />
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
      <Stamp at={at} className="armada-session-did__at" />
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

/** Held by the agent's CLI until the call its turn waits on returns. Gone once the agent has read it. */
function Queued() {
  const said = "Queued until the subagent returns";
  return (
    <Tooltip label={said}>
      <span className="armada-session-you__queued" role="img" aria-label={said}>
        <Clock size={12} strokeWidth={2} aria-hidden />
        <span className="armada-session-you__queued-word">Queued</span>
      </span>
    </Tooltip>
  );
}

/** What a message of the person's draws from. */
type YourRow = Pick<Extract<SessionThreadRow, { kind: "message"; from: "you" | "agent" }>, "id" | "at" | "text" | "files" | "sketches" | "tags" | "delivery" | "queued">;

/**
 * The person's own message: the band with its Delivery mark, the words, and a Retry where it did not go.
 * **One row wherever words are sent**: a Session's thread, Helm's, and the box under a Drone's log draw this.
 */
export function YourMessage({ row, onRetry, forkable = false }: { row: YourRow; onRetry?: (rowId: string) => void; forkable?: boolean }) {
  // Where the host can open what a chip in a row names (Helm's thread links them), a tag is a link.
  const refs = useContext(ProseRefs);
  const openTag = refs === null ? undefined : (tag: ComposerTag): void => (tag.kind === "branch" ? undefined : refs.open({ kind: tag.kind, id: tag.id, title: titleWithoutReference(tag) }));
  const { said, debug } = splitDebugInfo(row.text);
  const placed = placeTags(said, row.tags ?? [], openTag);
  // Notes sent from a viewer: what each is about opens it again.
  const opens = useContext(NotedOpens);
  const noted = opens === null || placed.parts !== null ? undefined : notedOf(said);
  return (
    <li className="armada-session-you" role="region" aria-label="Your message" data-delivery={row.delivery}>
      <div className="armada-session-you__band">
        <span className="armada-session-you__who">You</span>
        <Delivery row={row} />
        {row.queued === true ? <Queued /> : null}
        <RowStamp row={row} className="armada-session-you__at" />
      </div>
      <div className="armada-session-you__body" {...(forkable ? { [FORK_ROW]: row.id } : {})}>
        {placed.parts !== null ? (
          <p className="armada-session-said">{placed.parts}</p>
        ) : (
          // Run is on what the agent wrote, not on what the person did.
          <ProseRunner.Provider value={null}>
            <ProseCommand.Provider value={null}>
              {noted === undefined || opens === null ? <Prose text={said} /> : <NotedMessage blocks={noted} opens={opens} />}
            </ProseCommand.Provider>
          </ProseRunner.Provider>
        )}
        {debug === undefined || debug === "" ? null : (
          <details className="armada-session-you__debug">
            <summary className="armada-session-you__debug-head">
              <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-you__debug-chevron" />
              Debug info
            </summary>
            <ProseRunner.Provider value={null}>
              <ProseCommand.Provider value={null}>
                <Prose text={debug} />
              </ProseCommand.Provider>
            </ProseRunner.Provider>
          </details>
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

/** Another Job's message, or Fleet's notice. A notice with `onMuteNotice` opens a Disable header over three choices on a right press, or on the ContextMenu key or Shift+F10. */
function FromSession({
  row,
  forkable,
  disputed,
  onOpenSession,
  onMuteNotice,
  onTeach,
}: {
  row: Extract<SessionThreadRow, { kind: "message"; from: "session" }>;
  forkable: boolean;
  disputed: boolean;
  onOpenSession: (id: string) => void;
  onMuteNotice?: (notice: SessionNotice, choice: NoticeChoice) => void;
  onTeach?: (pick: TeachPick) => void;
}) {
  const { sender, notice } = row;
  const [menu, setMenu] = useState<{ x: number; y: number; row: HTMLElement | null } | undefined>(undefined);
  const muting = notice !== undefined && onMuteNotice !== undefined;
  const teaching = sender.id === "fleet" && onTeach !== undefined;
  return (
    <li
      className="armada-session-from"
      role="region"
      aria-label={`Message from ${sender.title}`}
      {...(!muting && !teaching
        ? {}
        : {
            onContextMenu: (event: ReactMouseEvent<HTMLLIElement>) => {
              event.preventDefault();
              setMenu({ x: event.clientX, y: event.clientY, row: event.currentTarget.querySelector<HTMLElement>(".armada-session-from__band") });
            },
            onKeyDown: (event: KeyboardEvent<HTMLLIElement>) => {
              if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
              event.preventDefault();
              const box = event.currentTarget.getBoundingClientRect();
              setMenu({ x: box.left + box.width / 2, y: box.bottom, row: event.currentTarget.querySelector<HTMLElement>(".armada-session-from__band") });
            },
          })}
    >
      <button type="button" className="armada-session-from__band" aria-label={`Open Job ${sender.title}`} onClick={() => onOpenSession(sender.id)}>
        <SquareTerminal size={12} strokeWidth={2} aria-hidden />
        <span className="armada-session-from__id">{sender.id}</span>
        <span className="armada-session-from__title">{sender.title}</span>
        {disputed ? <DisputedMark /> : null}
        <Stamp still at={row.at} className="armada-session-from__at" />
      </button>
      <div className="armada-session-from__body" {...(forkable ? { [FORK_ROW]: row.id } : {})}>
        <Prose text={row.text} />
      </div>
      {menu === undefined || (!muting && !teaching) ? null : (
        <DropdownMenu
          triggerLabel="Fleet notice"
          at={{ x: menu.x, y: menu.y }}
          open
          entries={noticeEntries(notice, muting, teaching)}
          onOpenChange={(open) => {
            if (open) return;
            setMenu(undefined);
            menu.row?.focus();
          }}
          onSelect={(chosen) => {
            if (chosen === "teach") onTeach?.({ kind: "notice", text: row.text, at: row.at, ...(notice === undefined ? {} : { ids: [notice] }) });
            else if (notice !== undefined) onMuteNotice?.(notice, chosen as NoticeChoice);
          }}
        />
      )}
    </li>
  );
}

function Row({
  row,
  onOpenSession,
  onOpenWindow,
  onOpenSketch,
  onOpenFile,
  onDraft,
  onWithdrawDraft,
  onAnswerSketch,
  sessionId,
  agentName = "Agent",
  onOpenRetro,
  onOpenSent,
  onRetry,
  forkable,
  disputed = false,
  onMuteNotice,
  onTeach,
}: {
  row: Exclude<SessionThreadRow, { kind: "tool" | "check" }>;
  onDraft?: (rowId: string, answer: DraftAnswer) => void;
  onWithdrawDraft?: (rowId: string, reason?: string) => void;
  onAnswerSketch?: (sketch: string, answer: SketchAnswer) => void;
  sessionId?: string;
  onRetry?: (rowId: string) => void;
  forkable: boolean;
  /** A later row disputes this one, so Fleet's card draws the Disputed mark. */
  disputed?: boolean;
  onMuteNotice?: (notice: SessionNotice, choice: NoticeChoice) => void;
  onTeach?: (pick: TeachPick) => void;
  onOpenSession: (id: string) => void;
  onOpenWindow?: (url: string) => void;
  onOpenSketch?: (sketch: string) => void;
  onOpenFile?: (path: string) => void;
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
        <RowStamp row={row} className="armada-session-woke__at" />
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
        <RowStamp row={row} className="armada-session-act__at" />
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
  if (row.kind === "file") {
    return (
      <li className="armada-session-fold">
        <button type="button" className="armada-session-fold__head armada-session-fold__head--press" aria-label={`Opened ${row.name}`} onClick={() => onOpenFile?.(row.path)}>
          <File size={12} strokeWidth={2} aria-hidden />
          Opened <span className="mono">{row.name}</span>
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
  if (row.kind === "draft") return <DraftRow row={row} keep={sessionId === undefined ? `draft:${row.id}` : `draft:${sessionId}:${row.id}`} {...(onDraft === undefined ? {} : { onAnswer: onDraft })} {...(onWithdrawDraft === undefined ? {} : { onWithdraw: onWithdrawDraft })} />;
  if (row.kind === "requested") return <RequestedRow row={row} agentName={agentName} />;
  if (row.kind === "saved") return <SavedRow row={row} />;
  if (row.kind === "sketch") return <SketchRow row={row} keep={sessionId === undefined ? `sketch:${row.id}` : `sketch:${sessionId}:${row.id}`} {...(onOpenSketch === undefined ? {} : { onOpen: onOpenSketch })} {...(onAnswerSketch === undefined ? {} : { onAnswer: onAnswerSketch })} />;
  if (row.kind === "started") return <StartedRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "moved") return <MovedRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "divider") return <li className="armada-session-divider" role="separator" />;
  if (row.kind === "waiter") return <WaiterRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "linked") return <LinkedRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "disputed") return <DisputedRow row={row} onOpen={onOpenSession} />;
  if (row.kind === "change") {
    return (
      <li className="armada-session-change" role="region" aria-label={`${row.what} changed`}>
        <span className="armada-session-lease__eyebrow">{row.what}</span>
        <span className="armada-session-change__facts">
          {row.from}
          <ArrowRight size={12} strokeWidth={2} aria-hidden />
          {row.to}
        </span>
        <RowStamp row={row} className="armada-session-lease__at" />
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
        <RowStamp row={row} className="armada-session-lease__at" />
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
        <RowStamp row={row} className="armada-session-lease__at" />
      </li>
    );
  }
  if (row.kind === "lease" || row.kind === "release") {
    const said = row.kind === "lease" ? "Slot leased" : "Slot released";
    return (
      <li className="armada-session-lease" role="region" aria-label={said}>
        <span className="armada-session-lease__eyebrow">{said}</span>
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
        <RowStamp row={row} className="armada-session-lease__at" />
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
          <RowStamp row={row} className="armada-session-you__at" />
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
    return <FromSession row={row} forkable={forkable} disputed={disputed} onOpenSession={onOpenSession} {...(onMuteNotice === undefined ? {} : { onMuteNotice })} {...(onTeach === undefined ? {} : { onTeach })} />;
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
        <RowStamp row={row} className="armada-helm-thread__at" />
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

/** The answer that allows a kind of call, drawn in the Calls like row and only where the ask names its kind. */
const LIKE_THIS = "allow_like_this_for_session";

/** The answer that refuses a permission: the card's no. */
const REFUSE = "refuse";

/**
 * A call's kind as its words, `node drive.mjs`, each a press, the way `PathPicker` draws a path:
 * the word chosen and every word after it are lit, so a press on the program reads as covering any
 * script. A kind with no script is its program alone.
 */
function KindPicker({ kind, wide, onWiden }: { kind: CallKind; wide: boolean; onWiden: (wide: boolean) => void }) {
  const words = kind.script === undefined ? [{ word: kind.program, wide: true }] : [{ word: kind.program, wide: true }, { word: kind.script, wide: false }];
  return (
    <div className="armada-session-thread__kind" role="group" aria-label="Calls like">
      {words.map((one) => (
        <button
          key={one.word}
          type="button"
          className="armada-session-thread__kind-word"
          aria-pressed={one.wide === wide || words.length === 1}
          data-covered={wide && !one.wide ? true : undefined}
          onClick={() => onWiden(one.wide)}
        >
          {one.word}
        </button>
      ))}
    </div>
  );
}

/** A card's answers: the ones for this call in a row, then **Always** in a panel of its own, the
 * folder picker inside it so the choice reads as what Always allow and Always block cover. Keyed
 * by the path, so a new ask opens on its whole path. A card that names its call's kind adds Calls
 * like under it, whose answer can be narrowed to the same folder. */
function ReachAnswers({ reaches, reachesTo, kind, offers, onAnswer, keyed = false }: { reaches: string; reachesTo?: "read" | "write"; kind?: CallKind; offers: readonly AskOffer[]; onAnswer: SessionThreadProps["onAnswer"]; /** The card is the thread's target: its refuse draws `⌥X`. */ keyed?: boolean }) {
  const [folder, setFolder] = useState(reaches);
  const [wide, setWide] = useState(false);
  // Off at the start: the kind alone is the answer's own width, and a folder narrows it only when asked.
  const [onlyIn, setOnlyIn] = useState(false);
  const shown = offers.filter((offer) => offer.id !== "allow_once" && offer.id !== LIKE_THIS);
  const once = shown.filter((offer) => !FOLDER_ANSWERS.has(offer.id));
  const always = shown.filter((offer) => FOLDER_ANSWERS.has(offer.id));
  const like = kind === undefined ? undefined : offers.find((offer) => offer.id === LIKE_THIS);
  const chosenKind: CallKind | undefined = kind === undefined ? undefined : wide || kind.script === undefined ? { program: kind.program } : { program: kind.program, script: kind.script };
  const name = folder.split("/").filter((part) => part.length > 0).pop() ?? folder;
  const covers = folder === reaches ? reaches : `${folder}/*`;
  return (
    <div className="armada-session-thread__reach" role="group" aria-label="Answers">
      <div className="armada-session-thread__answers">
        {once.map((offer) => (
          <Tooltip key={offer.id} label={offer.means}>
            <Button size="sm" variant="secondary" onClick={() => onAnswer(offer.id)}>
              {offer.label}
              <ThreadKbd act="no" on={keyed && offer.id === REFUSE} />
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
        {like === undefined || kind === undefined ? null : (
          <div className="armada-session-thread__like">
            <span className="armada-session-thread__always-head">Calls like</span>
            <KindPicker kind={kind} wide={wide} onWiden={setWide} />
            <div className="armada-session-thread__answers">
              <Switch size="sm" checked={onlyIn} onChange={(event) => setOnlyIn(event.target.checked)}>
                Only in <span className="armada-session-thread__always-folder">{name}</span>
              </Switch>
              <Tooltip label={like.means}>
                <Button size="sm" variant="secondary" onClick={() => onAnswer(like.id, undefined, onlyIn ? folder : undefined, chosenKind)}>
                  {like.label}
                </Button>
              </Tooltip>
            </div>
          </div>
        )}
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
  const aside = approval.setAside === true && !approved;
  // A plan set aside folds to a chip, at the press and on arriving so; its press unfolds it.
  const [folded, setFolded] = useState(aside);
  useEffect(() => {
    if (aside) setFolded(true);
  }, [aside]);
  // The groups, with their subagent types, paths and task marks, fold under Details where the plan has a summary to read first.
  const [detailed, setDetailed] = useState(false);
  const plain = approval.summary !== undefined;
  const Fold = folded ? ChevronRight : ChevronDown;
  const revising = approval.revision !== undefined && !approved;
  const unchanged = useUnchanged(revising ? approval.revision : undefined);
  const stateSaid = approval.going === true ? "Sending" : approved ? "Plan approved" : aside ? "Plan set aside" : "Plan waiting for approval";
  const StateGlyph = approval.going === true ? LoaderCircle : approved ? BadgeCheck : BellRing;
  const review = onReview === undefined || folded ? undefined : { tabIndex: 0, "data-reviewable": "", onClick: (event: ReactMouseEvent<HTMLElement>) => { if ((event.target as Element).closest("button") === null) onReview(); }, onKeyDown: (event: KeyboardEvent<HTMLElement>) => { if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); onReview(); } } };
  const adversarial = approved ? undefined : approval.adversarial;
  const findingsHere = adversarial !== undefined && findingsOf(adversarial.review).length > 0;
  // Pinned to the plan: which finding is about what, and the keys that run down them wherever they stand.
  const pinned = adversarial !== undefined && !folded;
  const anchors = pinned && adversarial !== undefined ? anchorsOf(adversarial.review, approval.groups.flatMap((group) => [group.id, ...group.tasks.map((task) => task.id)])) : undefined;
  const keys = useFindingKeys(adversarial?.review, { onSend: (id) => adversarial?.onSend(id), onDismiss: (id) => adversarial?.onDismiss(id) });
  const acts = { onSend: keys.send, onDismiss: keys.dismiss };
  // U opens the unchanged groups of a revision, or closes them all; the registry's `plan_adversarial_review` runs an adversarial review (or runs it again); wherever focus is on the card.
  const onKeys = (event: KeyboardEvent<HTMLElement>) => {
    const plain = !event.metaKey && !event.ctrlKey && !event.altKey;
    if (!folded && revising && event.key === "u" && plain) {
      event.preventDefault();
      unchanged.toggleAll();
    } else if (!folded && adversarial !== undefined && isPressed("plan_adversarial_review", event) && (adversarial.review.state === "idle" || adversarial.review.state === "failed" || adversarial.review.state === "stale")) {
      event.preventDefault();
      adversarial.onRun();
    } else review?.onKeyDown(event);
  };
  // While it waits, one of the Job's waiting cards (`thread-keys.tsx`): `⌥↵` Approve, `⌥E` the plan in full to suggest in, `⌥R` Not yet.
  const waiting = !approved && !aside && approval.going !== true;
  const { target, bind } = useThreadCard(
    !waiting
      ? undefined
      : {
          id: "plan",
          kind: "Plan",
          title: approval.groups.map((group) => group.title).join(", "),
          acts: {
            yes: { verb: "Approve", run: approval.onApprove },
            ...(onReview === undefined ? {} : { change: { verb: "Suggest", run: onReview } }),
            ...(approval.onSetAside === undefined ? {} : { sendBack: { verb: "Not yet", run: approval.onSetAside } }),
          },
        },
  );
  const groups = (
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
          {!plain || group.paths === undefined || group.paths.length === 0 ? null : (
            <span className="armada-session-thread__paths" aria-label="Paths">
              {group.paths.map((path) => (
                <code key={path}>{path}</code>
              ))}
            </span>
          )}
          {anchors === undefined || plain ? null : <GroupFindings group={group} anchors={anchors} {...acts} />}
        </li>
      ))}
    </ul>
  );
  const technical = revising ? <PlanRevisionView revision={approval.revision!} unchanged={unchanged} /> : groups;
  return (
    <Card flat className="armada-session-thread__ask armada-session-thread__plan" role="article" aria-label={approved ? "Plan approved" : aside ? "Plan set aside" : "Plan to approve"} data-approved={approved || undefined} data-aside={aside || undefined} data-folded={folded || undefined} data-findings={findingsHere || undefined} {...bind} {...review} {...(revising || adversarial !== undefined ? { onKeyDown: onKeys } : {})}>
      <div className="armada-session-thread__plan-head">
        <button type="button" className="armada-session-thread__plan-band" aria-expanded={!folded} aria-label={aside ? "Plan set aside" : "Plan"} onClick={() => setFolded(!folded)}>
          <ListChecks size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-thread__eyebrow">{aside ? "Plan set aside" : "Plan"}</span>
          {approval.version === undefined ? null : <span className="armada-session-thread__plan-version">v{approval.version}</span>}
          {aside ? null : (
            <Tooltip label={stateSaid}>
              <span className="armada-session-thread__settled" role="img" aria-label={stateSaid} data-state={approval.going === true ? "going" : approved ? "approved" : "waiting"}>
                <StateGlyph size={16} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          )}
          <Fold className="armada-session-thread__plan-fold" size={12} strokeWidth={2} aria-hidden />
        </button>
        {approved || !folded || aside ? null : (
          <Button size="sm" variant="primary" onClick={approval.onApprove}>
            Approve
            <ThreadKbd act="yes" on={target} />
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
      <div className="armada-session-thread__plan-body" {...(pinned ? keys.bind : {})}>
        {plain ? (
          <div className="armada-session-thread__plain">
            <p className="armada-session-thread__plan-summary">{approval.summary}</p>
            {anchors === undefined ? null : <FindingNotes findings={anchors.whole} label="Findings on the plan" {...acts} />}
            <ul className="armada-session-thread__gives" aria-label="What each group gives">
              {approval.groups.map((group) => (anchors === undefined ? <li key={group.id}>{group.gives ?? group.title}</li> : <GivesRow key={group.id} text={group.gives ?? group.title} group={group} anchors={anchors} {...acts} />))}
            </ul>
            <Button size="sm" variant="ghost" aria-expanded={detailed} onClick={() => setDetailed(!detailed)}>
              {detailed ? "Show less" : "Details"}
            </Button>
            {detailed ? technical : null}
          </div>
        ) : anchors === undefined ? technical : (
          <div className="armada-session-thread__pinned">
            <FindingNotes findings={anchors.whole} label="Findings on the plan" {...acts} />
            {technical}
          </div>
        )}
        {approval.sketch === undefined ? null : (
          <ul className="armada-session-thread__sketch">
            <SketchRow row={{ id: approval.sketch.id, at: "", kind: "sketch", sketch: approval.sketch.id, title: approval.sketch.title, drawing: approval.sketch.scene }} onOpen={() => onReview?.()} />
          </ul>
        )}
      </div>
      {adversarial?.review.state !== "failed" ? null : (
        <div className="armada-session-thread__plan-review" onClick={(event) => event.stopPropagation()}>
          <ReviewFailed said={adversarial.review.said} />
        </div>
      )}
      {approved ? null : (
        <div className="armada-session-thread__answers" role="group" aria-label="Answers">
          <Button size="sm" variant="primary" onClick={approval.onApprove}>
            Approve
            <ThreadKbd act="yes" on={target} />
          </Button>
          {/* Opens the plan in full, where a group or task takes a suggestion: what `⌥E` does. */}
          {aside || onReview === undefined ? null : (
            <Button size="sm" variant="secondary" onClick={onReview}>
              Suggest
              <ThreadKbd act="change" on={target} />
            </Button>
          )}
          {aside || approval.onSetAside === undefined ? null : (
            <Button size="sm" variant="secondary" onClick={approval.onSetAside}>
              Not yet
              <ThreadKbd act="sendBack" on={target} />
            </Button>
          )}
          {adversarial === undefined ? null : <AdversarialReviewPress review={adversarial.review} onRun={adversarial.onRun} {...(adversarial.shortcut === undefined ? {} : { shortcut: adversarial.shortcut })} />}
        </div>
      )}
        </>
      )}
    </Card>
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
  // One of the Job's waiting cards (`thread-keys.tsx`): `⌥↵` End job, `⌥X` Keep going.
  const { target, bind } = useThreadCard({ id: "done", kind: "Done", title: "Nothing left to do", acts: { yes: { verb: "End job", run: done.onEnd }, no: { verb: "Keep going", run: done.onKeep } } });
  return (
    <>
          {done.made === undefined || done.made.length === 0 ? null : (
            <ul className="armada-session-done__made" aria-label={done.made[0]!.kind === "pull_request" ? "Pull requests" : "Artifacts"}>
              {done.made.map((row) => {
                const form = artifactOf(row);
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
          <div className="armada-session-done__acts" {...bind}>
            <Button size="sm" variant="secondary" onClick={done.onEnd}>
              End job
              <ThreadKbd act="yes" on={target} />
            </Button>
            <Button size="sm" variant="ghost" onClick={done.onKeep}>
              Keep going
              <ThreadKbd act="no" on={target} />
            </Button>
          </div>
    </>
  );
}

/** A judge's concern as a card: where it came from, the claim, what it cites, and the two ways on. `bare` leaves the band off, for a panel that already says what it is. */
export function ConcernCard({ concern: one, onSend, onDismiss, bare }: { concern: ThreadConcern; onSend?: () => void; onDismiss?: () => void; bare?: true }) {
  // One of the Job's waiting cards (`thread-keys.tsx`): `⌥↵` Send to job, `⌥X` Dismiss.
  const { target, bind } = useThreadCard({
    id: `concern:${one.id}`,
    kind: "Judge",
    title: one.text,
    acts: { ...(onSend === undefined ? {} : { yes: { verb: "Send to job", run: onSend } }), ...(onDismiss === undefined ? {} : { no: { verb: "Dismiss", run: onDismiss } }) },
  });
  return (
    <Card flat className="armada-session-thread__ask armada-session-thread__concern" role="group" aria-label={`Judge: ${one.text}`} {...bind}>
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
      <div className="armada-session-thread__answers" role="group" aria-label="Answers">
        <Tooltip label="Goes to the agent as your message">
          <Button size="sm" variant="secondary" onClick={onSend}>
            Send to job
            <ThreadKbd act="yes" on={target} />
          </Button>
        </Tooltip>
        <Button size="sm" variant="ghost" onClick={onDismiss}>
          Dismiss
          <ThreadKbd act="no" on={target} />
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
export function AskCards({ asked, askRef, onAnswer, lapsed, flat, keep }: Pick<SessionThreadProps, "asked" | "askRef" | "onAnswer" | "lapsed"> & { /** Inside a panel that frames it: the question deck without its own card, band or fold. */ flat?: true; /** What the ask is about: its answers are kept under it until it is answered. */ keep?: string }) {
  // A permission is one of the Job's waiting cards (`thread-keys.tsx`): `⌥↵` Allow once, `⌥X` its refuse. A question's deck registers itself.
  const permission = asked !== undefined && (asked.questions?.length ?? 0) === 0 ? asked : undefined;
  const refuse = permission?.offers?.find((offer) => offer.id === REFUSE);
  const { target, bind } = useThreadCard(
    permission === undefined
      ? undefined
      : {
          id: `permission:${permission.command}`,
          kind: permission.reachesTo === undefined ? "Permission" : `Permission to ${permission.reachesTo}`,
          title: permission.command.split("\n")[0] ?? "",
          acts: {
            ...(permission.offers === undefined ? { yes: { verb: "Allow once", run: () => onAnswer() } } : allowsOnce(permission) ? { yes: { verb: "Allow once", run: () => onAnswer("allow_once") } } : {}),
            ...(permission.offers === undefined ? { no: { verb: "Deny", run: () => onAnswer() } } : refuse === undefined ? {} : { no: { verb: refuse.label, run: () => onAnswer(REFUSE) } }),
          },
        },
  );
  const cardRef = (element: HTMLDivElement | null) => {
    bind.ref(element);
    if (typeof askRef === "function") askRef(element);
    else if (askRef !== undefined && askRef !== null) (askRef as { current: HTMLDivElement | null }).current = element;
  };
  return (
    <>
      {asked === undefined ? null : (
        <Card flat {...bind} ref={cardRef} className="armada-session-thread__ask" role="article" aria-label="Waiting on you">
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
                  <ThreadKbd act="yes" on={target} />
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
              {...(keep === undefined ? {} : { keep })}
              questions={asked.questions}
              onAnswer={onAnswer}
              canSkip={asked.offers === undefined || asked.offers.some((offer) => offer.id === "refuse")}
            />
          ) : asked.reaches !== undefined && asked.offers?.some((offer) => FOLDER_ANSWERS.has(offer.id)) === true ? (
            <ReachAnswers key={asked.reaches} keyed={target} reaches={asked.reaches} {...(asked.reachesTo === undefined ? {} : { reachesTo: asked.reachesTo })} {...(asked.kind === undefined ? {} : { kind: asked.kind })} offers={asked.offers} onAnswer={onAnswer} />
          ) : (
          <div className="armada-session-thread__answers" role="group" aria-label="Answers">
            {asked.offers === undefined ? (
              <>
                <Button size="sm" variant="secondary" onClick={() => onAnswer()}>
                  Allow once
                  <ThreadKbd act="yes" on={target} />
                </Button>
                <Button size="sm" variant="secondary" onClick={() => onAnswer()}>
                  Deny
                  <ThreadKbd act="no" on={target} />
                </Button>
              </>
            ) : (
              asked.offers.filter((offer) => offer.id !== "allow_once").map((offer) => (
                <Tooltip key={offer.id} label={offer.means}>
                  <Button size="sm" variant="secondary" onClick={() => onAnswer(offer.id)}>
                    {offer.label}
                    <ThreadKbd act="no" on={target && offer.id === REFUSE} />
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

export function SessionThread({ rows, onRetry, approval, asked, askRef, onAnswer, lapsed, waitingOnYou, workingOn, onOpenSession, onOpenWindow, onOpenSketch, onOpenFile, onDraft, onWithdrawDraft, onAnswerSketch, onOpenCheck, onOpenSubagent, onOpenRetro, onOpenSent, sessionId, working, onStop, agentName, runner, onCommand, commandNames, onForkAt, onReplyAt, onTeachAt, onTeachRow, onMuteNotice, lead, trailing, transcript, noted, inline }: SessionThreadProps) {
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
  // **A message the person sends brings the thread to its end**, from the box or from a viewer's notes, wherever he was reading.
  const mine = useRef<{ session: string | undefined; key: string | undefined }>(undefined);
  useLayoutEffect(() => {
    let key: string | undefined;
    for (let at = rows.length - 1; at >= 0 && key === undefined; at -= 1) {
      const row = rows[at]!;
      if (row.kind === "message" && row.from === "you") key = row.stableKey ?? row.id;
    }
    const was = mine.current;
    if (was !== undefined && was.session === sessionId && key !== undefined && key !== was.key) pinned.current = true;
    mine.current = { session: sessionId, key };
  }, [rows, sessionId]);
  useLayoutEffect(() => {
    const one = scroller.current;
    if (one !== null && pinned.current) one.scrollTop = one.scrollHeight;
  }, [rows, sessionId, working, approval === undefined ? 0 : approval.approved ? 2 : 1, asked?.command ?? ""]);
  // The trailing entry grows with what is written in it, which no row of the thread's announces.
  const tail = useRef<HTMLLIElement>(null);
  const hasTrailing = trailing !== undefined;
  useLayoutEffect(() => {
    const one = scroller.current;
    const entry = tail.current;
    if (one === null || entry === null) return;
    const follow = (): void => {
      if (pinned.current) one.scrollTop = one.scrollHeight;
    };
    follow();
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(follow);
    watch.observe(entry);
    return () => watch.disconnect();
  }, [hasTrailing, sessionId]);
  const disputes = useMemo(() => new Set(rows.flatMap((row) => (row.kind === "disputed" ? [row.notice] : []))), [rows]);
  const asking = (approval !== undefined && !approval.approved) || asked !== undefined || lapsed !== undefined;
  // A turn that ends in a question for the person stays open in the thread.
  const all = useMemo(() => {
    const folded = itemsOf(rows, disputes);
    const last = folded[folded.length - 1];
    const turn = last?.kind === "fleetTurn" ? last : folded[folded.length - 2]?.kind === "fleetTurn" && last?.kind === "row" && last.row.kind === "message" && last.row.from === "agent" ? folded[folded.length - 2] : undefined;
    return (asking || waitingOnYou === true) && turn?.kind === "fleetTurn" ? itemsOf(rows, new Set([...disputes, turn.id])) : folded;
  }, [rows, disputes, asking, waitingOnYou]);
  const [drawn, setDrawn] = useState(DRAWN);
  useLayoutEffect(() => {
    setDrawn(DRAWN);
  }, [sessionId]);
  const hidden = Math.max(0, all.length - drawn);
  const items = hidden === 0 ? all : all.slice(hidden);
  const endsInCalls = items[items.length - 1]?.kind === "calls";
  const cardsAfter = (at: number): ThreadCard[] => (inline ?? []).filter((card) => afterWhich(items, card) === at);
  // What waits on him is the thread's last card, answered in place.
  const cards = (
    <>
      {approval === undefined || approval.approved ? null : <PlanCard approval={approval} />}
      <AskCards {...(asked === undefined ? {} : { asked })} {...(askRef === undefined ? {} : { askRef })} onAnswer={onAnswer} {...(lapsed === undefined ? {} : { lapsed })} />
    </>
  );
  return (
    <div className="armada-session-thread" ref={whole}>
      {onForkAt === undefined && onReplyAt === undefined && onTeachAt === undefined ? null : <ForkFromSelection scope={whole} {...(onForkAt === undefined ? {} : { onFork: onForkAt })} {...(onReplyAt === undefined ? {} : { onReply: onReplyAt })} {...(onTeachAt === undefined ? {} : { onTeach: onTeachAt })} />}
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
        <StampSurface>
        <NotedOpens.Provider value={noted ?? null}>
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
              <Calls key={item.id} rows={item.rows} working={working === true && !asking && at === items.length - 1} {...(onStop === undefined ? {} : { onStop })} {...(onTeachRow === undefined ? {} : { onTeach: onTeachRow })} />
            ) : item.kind === "fleetTurn" ? (
              <FleetTurn key={item.id} notice={item.notice} reply={item.reply} handled={item.handled}{...(onMuteNotice === undefined ? {} : { onMuteNotice })} {...(onTeachRow === undefined ? {} : { onTeach: onTeachRow })} />
            ) : item.kind === "subagent" ? (
              <SubagentRow key={item.id} subagent={item.subagent} {...(onOpenSubagent === undefined ? {} : { onOpen: onOpenSubagent })} />
            ) : item.kind === "checks" ? (
              <CheckRuns key={item.id} rows={item.rows} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} {...(onTeachRow === undefined ? {} : { onTeach: onTeachRow })} />
            ) : (
              <Row key={"stableKey" in item.row && item.row.stableKey !== undefined ? item.row.stableKey : item.row.id} row={item.row} {...(onRetry === undefined ? {} : { onRetry })} onOpenSession={onOpenSession} onOpenWindow={onOpenWindow} onOpenSketch={onOpenSketch} {...(onOpenFile === undefined ? {} : { onOpenFile })} {...(onDraft === undefined ? {} : { onDraft })} {...(onWithdrawDraft === undefined ? {} : { onWithdrawDraft })} {...(onAnswerSketch === undefined ? {} : { onAnswerSketch })} {...(sessionId === undefined ? {} : { sessionId })} onOpenRetro={onOpenRetro} onOpenSent={onOpenSent} forkable={onForkAt !== undefined || onReplyAt !== undefined || onTeachAt !== undefined} disputed={disputes.has(item.row.id)} {...(onMuteNotice === undefined ? {} : { onMuteNotice })} {...(onTeachRow === undefined ? {} : { onTeach: onTeachRow })} {...(agentName === undefined ? {} : { agentName })} />
            ),
            approval?.approved === true && item.kind === "row" && item.row.id === approval.after ? <li key="plan-approved" className="armada-session-thread__record"><PlanCard approval={approval} /></li> : null,
            ...cardsAfter(at).map((card) => (
              <li key={card.key} className="armada-session-thread__card">
                {card.node}
              </li>
            )),
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
          {working === true && (!endsInCalls || asking) ? (
            <li className="armada-session-thread__working">
              <SessionMark state="working" said="Working" />
              {onStop === undefined ? null : <SessionStop onStop={onStop} />}
            </li>
          ) : null}
          {trailing === undefined ? null : (
            <li ref={tail} className="armada-session-thread__trailing">
              {trailing}
            </li>
          )}
        </ol>
        </ProseCommand.Provider>
        </ProseRunner.Provider>
        </ProseWalk.Provider>
        </NotedOpens.Provider>
        </StampSurface>
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
