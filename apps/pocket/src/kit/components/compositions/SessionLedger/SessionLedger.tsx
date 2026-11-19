import type { ReactNode } from "react";
import type { LedgerPullRequest } from "./PullRequestMarks";
import type { LinkedSessionProps } from "../LinkedSession/LinkedSession";
import type { HeldSlotsProps } from "../HeldSlots/HeldSlots";
import type { CodeRunner } from "../../primitives/Prose/RunnableBlock";
import type { LucideIcon } from "lucide-react";

/**
 * What a Session has accumulated, a section per kind and a row per thing.
 * **Every row is a press that opens what it names**: the slot on Cleanup, the
 * pull request in a sheet, the Studio on its whiteboard,
 * a sketch beside it, a page or a document at its address, a file on the
 * machine. The ledger draws no surface of its own; the host says where each
 * press goes.
 *
 * **A branch row ends in the chip of the slot it stands in**, so slots have no section of their own:
 * a slot no branch is on keeps a row of its own, drawn last under Branches with the key glyph (the
 * owner's choice of 16 Oct 2026, "slot chip on branch").
 *
 * **What waits on the person is not here.** The thread's Needs you band holds it, in every state of the
 * Session, done included (the owner's choice of 16 Oct 2026: "its already in the thread"); `LedgerRows`
 * and `waitingMark` are what that band draws it with.
 *
 * **A section is drawn only when it holds a row.** A ledger with nothing in it
 * draws one small picture and no words (the owner's choice of 7 Oct 2026, who
 * first asked for every section dim and then took it back: once some filled,
 * the empty ones only took room).
 */
export type LedgerKind =
  | "waiting"
  | "slot"
  | "branch"
  | "pull_request"
  | "studio"
  | "sketch"
  | "subagent"
  | "background"
  | "artifact"
  | "fork"
  | "start"
  | "linked"
  | "closed";

/** What an artifact is: a page published, a file written outside the code, a picture looked at, a Doc, or a page shown in a window. */
export type ArtifactForm = "page" | "file" | "image" | "doc" | "document" | "window";

/** What a background row is: a shell command the agent ran in the background, or a monitor it set to watch. */
export type BackgroundForm = "command" | "monitor";

/** A hue a ledger row or a card's fact carries: the Checks or the turn it stands for, never a chosen colour. */
export type SessionLedgerTone =
  | "pending"
  | "passed"
  | "failed"
  | "running"
  | "done"
  | "escalated"
  | "piloted"
  | "stopped";

/**
 * What hovering or focusing a row shows beside it: what the row is, one fact a line. **Built by the host
 * from what it already holds**, so the card draws nothing the window would have to go and read.
 */
export type SessionLedgerCard = {
  title: string;
  /** The item's id, as the card's head says it: `#2301`. */
  where?: string;
  /** Its state, where the host names the glyph; else a fact labelled State is drawn as the state band. */
  state?: {
    said: string;
    Glyph?: LucideIcon;
    tone?: SessionLedgerTone;
  };
  /**
   * `mono` is a machine value, a branch or a path. `tone` is drawn in the hue the row's mark takes.
   * A value of `{ at }` is an RFC 3339 instant, drawn as a stamp in the owner's time format, or as an age first with `age`. `open` draws the value as a link
   * to what it names, such as the Job that holds a pull request.
   */
  facts: readonly {
    label: string;
    value:
      | string
      | {
          at: string;
          age?: true;
        };
    tone?: SessionLedgerTone;
    mono?: boolean;
    open?: () => void;
  }[];
};

export type LedgerEntry = {
  key: string;
  kind: LedgerKind;
  /** The row's accessible name: what it is, and for a pull request how its Checks stand. */
  name: string;
  text: ReactNode;
  /** A `pull_request` row's state and Checks, drawn as the two bare marks the done callout draws too. */
  pr?: LedgerPullRequest;
  /** A pull request's Checks, or a subagent's turn, as the one mark at the row's end. */
  mark?: {
    glyph: SessionLedgerTone;
    said: string;
  };
  /** The preview a hover or a focus shows beside the row. Absent draws none. */
  card?: SessionLedgerCard;
  /** Which of the four an `artifact` row is, and so its glyph and what its tooltip names. */
  artifact?: ArtifactForm;
  /** An `artifact` file the agent showed and did not write: its glyph names it shown, not written. */
  shown?: true;
  /** Which of the two a `background` row is, and so its glyph and what its tooltip names. */
  background?: BackgroundForm;
  /** A `linked` row: a Session this one follows, drawn as a press that opens it. */
  linked?: LinkedSessionProps;
  /** The slot a branch stands in, as a chip at the row's end. */
  slot?: number;
  /**
   * Done with: a merged pull request, a finished subagent, an ended background command. Such a row is drawn only
   * under **All**, and a section that holds one gets the `Open | All` toggle.
   */
  finished?: boolean;
  /** A `waiting` row: what its press does, which is its glyph and what the glyph's tooltip names. */
  act?: WaitingActKind;
  /** A `waiting` row the Operator raised or would act on: its glyph is the Operator's, this its tooltip, and in the Needs you list its choices stand at its end for one press. */
  operator?: string;
  /** A `waiting` row for a question with choices: drawn as the question deck, the row's name its question, `onOther` taking the words of Other, and no Other without it. */
  options?: readonly {
    label: string;
    description?: string;
    onPick: () => void | Promise<unknown>;
  }[];
  onOther?: (text: string) => void | Promise<unknown>;
  /** A full pool's call: the slots it is held by, drawn under its choices with a Free on each. */
  held?: HeldSlotsProps;
  /** The procedure a question with choices asks the person to follow, drawn under its words and above the choices. */
  steps?: {
    lines: readonly string[];
    runner?: CodeRunner;
  };
  /**
   * A `waiting` row with no `act`: what the person types under it, sent as his answer. With this and no
   * `act` the row's press opens a reply box in place of doing something, and its glyph is Reply.
   */
  onReply?: (text: string) => void;
  /** A `waiting` row whose act is `run`, where the Session can run it: the row is the command's block with Run, as in the thread. */
  run?: {
    command: string;
    runner: CodeRunner;
    keep?: string;
  };
  /**
   * A `waiting` row whose act is a restart onto a build (`buildChangeOf`): the row is one key that
   * switches it, with nothing to copy and no command to read. `command` is its tooltip. Without a
   * `runner` the key does nothing.
   */
  restart?: {
    command: string;
    runner?: CodeRunner;
    keep?: string /** The key's own name where the row reads something else, as an offer does. */;
    label?: string;
  };
  /** What the reply box's draft is about (the Session and the item), so it survives closing the box and leaving the panel. */
  replyKeep?: string;
  /** A `waiting` row Fleet read out of the agent's reply rather than the agent stating it: where it asks, its glyph says so in place of Reply or Question. */
  fromReply?: boolean;
  /** A row read from a reply that asks nothing: the person says so at its end, and Fleet keeps the sentence as one that is no question. */
  onMisread?: () => void;
  /** Marked as a question, beside `onMisread`: Fleet keeps the sentence as an ask. */
  onConfirmed?: () => void;
  /** Fleet holds the row as confirmed: it draws no thumbs, and its menu offers only Not a question. */
  confirmed?: boolean;
  /**
   * A `waiting` row the agent stated: the person takes it off, with a reason for the agent that may be empty.
   * The row then draws Remove at its end, and its press opens the reason box.
   */
  onRemove?: (reason: string) => void;
  /** What the reason box's draft is about (the Session and the item), so it survives closing the box and leaving the panel. */
  removeKeep?: string;
  /** A `waiting` row he can put away in one press, with no reason box: a walk, a PR, anything the agent did not state itself. The row draws Dismiss at its end, shown on hover or focus. */
  onDismiss?: () => void;
  /**
   * What Dismiss is called on this row: `Skip` on a restart Fleet offers. Absent is Dismiss. A `restart` row with
   * `onDismiss` is such an offer: it reads its `text`, with the key and this act at its end.
   */
  dismissSaid?: string;
  /** Release at the row's end, shown on hover or focus, `name` what its tooltip names. Its press is the host's, which confirms first. */
  release?: {
    name: string;
    onPress: () => void;
  };
  /** Released from here: given back, or kept by Fleet with `kept` saying why. */
  released?: {
    kept?: string;
  };
  /** A held Job's row: Start at its end begins that Job without opening it. `starting` while the start waits on the request being read. */
  start?: {
    starting: boolean;
    onStart: () => void;
  };
  /** A `closed` row, an item he closed from Needs you: when, and Reopen, which puts it back there. Absent `onReopen` is an item whose thing has ended. */
  closed?: {
    at: string;
    onReopen?: () => void;
  };
  onOpen: () => void;
};

/** `message` is an item that waits on the person sending the message in the box. `steps` is a procedure, whose press opens its steps beside the thread. */
export type WaitingActKind = "walk" | "answer" | "approve_pr" | "run" | "message" | "steps";
