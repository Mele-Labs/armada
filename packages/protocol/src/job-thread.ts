// A Job's thread as a screen draws it, folded by Fleet: `get_job_thread` and `job_thread.tail`.
// Hand-mirrored from `crates/ipc/src/job_thread.rs`. `docs/practices/protocol/a-jobs-thread-fleet-folds.md`.
//
// **Fleet folds, every client draws.** An item carries what a row is and how it stands, never a sentence written for
// a screen. A sentence Fleet itself wrote, such as a notice or a Fleet act's `said`, passes through.
// The header rules in `protocol.ts` hold here: every closed set is left as `string`, with its words in the comment.

import type { AskedQuestion, CallInFlight } from "./calls";
import type { SessionRow, ToolSubagent } from "./hosted-sessions";
import type { JudgeArtifact } from "./judge";

/** What `get_job_thread` answers when it is asked for no `limit`. */
export const THREAD_ITEMS_SERVED = 200;
/** The most items one read answers; a larger `limit` is served this many. */
export const THREAD_ITEMS_MOST = 1000;
/** The most items from the end a change may reach for `job_thread.tail` to carry it; one further back is `job_thread.changed`. */
export const THREAD_TAIL_MOST = 200;

/** `get_job_thread` (`GET /sessions/thread?session_id=&before=&limit=&upto=`): the newest items of a Job's thread, or the ones before an item a client holds. */
export type JobThreadRead = {
  session_id: string;
  /** The id of an item, or of any row inside one: the answer holds the items before it. Absent is the newest. */
  before?: string;
  /** How many items, at most. Absent is `THREAD_ITEMS_SERVED`. */
  limit?: number;
  /**
   * The id of a row: the thread is read as it stood at that row, the rows after it left out before Fleet folds, so a
   * run of calls that went on past it ends at it. The read is not held and `job_thread.tail` never carries to it;
   * `before` and `limit` page the cut thread, and its `head` is empty. A row the thread does not hold is
   * refused 422 `fleet.thread_item_unknown`. Absent is the whole thread.
   */
  upto?: string;
};

/** The answer to `get_job_thread`. */
export type JobThread = {
  session_id: string;
  head: ThreadHead;
  /** Oldest first. **An item is never cut at the window's edge.** */
  items: ThreadItem[];
  /** Older items exist: pass this as `before` to read them. Absent where `items` starts the thread. */
  older?: string;
};

/**
 * `job_thread.tail`: what changed in a thread a client has read. `items` replace what a client holds from the item
 * `from` to the end; a client that does not hold `from` replaces everything it holds. `from` absent is `items`
 * after the last item, and with `items` empty it is the head alone changing.
 */
export type JobThreadTail = {
  session_id: string;
  from?: string;
  items: ThreadItem[];
  head: ThreadHead;
};

/**
 * `job_thread.changed`: a change reached back further than `THREAD_TAIL_MOST` items from the end, so Fleet sends the mark
 * and not the tail. A client that holds an item at or after `from` reads its window again; one that does not ignores it.
 */
export type JobThreadChanged = {
  session_id: string;
  from: string;
};

/** What stands over the whole thread rather than in it. */
export type ThreadHead = {
  /** The call the Job waits on the person for now, hosted or in a terminal. */
  asked?: ThreadAsk;
  /** The tool call the running turn is in: set while a turn runs and the thread's last row is a tool row. */
  open_call?: OpenCall;
  /** Questions nobody answered in time, that were refused or that were kept for the person, oldest first. Absent for an ended Job. */
  lapsed?: Lapsed;
  /** What Fleet did, does and will do for the Job, oldest first. Absent where none. */
  fleet_acts?: FleetAct[];
  trail: ThreadTrail;
  /** The plan put to the person, or set aside, drawn after the last item. **An approved plan is an item.** What it says is the record's `plan`. */
  plan?: ThreadPlan;
  /** The newest sketch put to the person and not answered or taken back, drawn after the plan. **Its newest card is no item** while it stands here. */
  sketch?: ThreadSketch;
};

/** The call the Job waits on, with the command a person reads: the argument whole where `detail` was cut, else `detail`, else the tool's name. */
export type ThreadAsk = {
  command: string;
  call: CallInFlight;
};

export type OpenCall = {
  text: string;
  since: string;
};

/** Questions that lapsed, under the id of the newest ask that held one. */
export type Lapsed = {
  id: string;
  questions: AskedQuestion[];
};

/** One thing Fleet did, does or will do for a Job, read from what is kept and never from the agent. */
export type FleetAct = {
  id: string;
  at: string;
  /** `over`, `under_way` or `coming`. */
  state: string;
  /** `nudge`, `step`, `slot`, `restart` or `told`. */
  kind: string;
  /** Fleet's own words: the first line of its note, or what it is about to do. */
  said: string;
  why?: string;
  until?: string;
};

/** Where the Job has been: when it started, the slots it held, and when each pull request opened. */
export type ThreadTrail = {
  started: string;
  /** Oldest first. Absent where it held none. */
  bays?: TrailBay[];
  /** By number, lowest first. Absent where it opened none. */
  opened?: TrailOpened[];
};

export type TrailBay = {
  slot: number;
  from: string;
  /** Absent while the Job still holds it. */
  until?: string;
};

export type TrailOpened = {
  number: number;
  at: string;
};

/** One thing a thread draws. */
export type ThreadItem =
  /**
   * A row drawn as it came. **Never a `tool`, `check`, `ask` or `window_ask` row**, which fold; a `plan` row, whose card is the
   * `plan` item; a `delivery` row, a `linked` row that `replaces`, or a `browser_reminder`. **A Fleet notice is a row only where it
   * is kept**: one the agent disputed, and the last Fleet turn while the Job waits on the person (its `asked`, or the head's `plan` or `sketch`).
   */
  | {
      kind: "row";
      row: SessionRow;
    }
  /**
   * Calls that ran one after another, without a subagent: `tool` rows, settled `ask` rows and `window_ask` rows. `names` are the tools it
   * holds, each once, in the order first called: an `ask` names the tool it asked about, a `window_ask` none.
   */
  | {
      kind: "calls";
      id: string;
      names: string[];
      rows: SessionRow[];
    }
  /** A call that started a subagent, alone. */
  | {
      kind: "subagent";
      id: string;
      at: string;
      subagent: ToolSubagent;
    }
  /** Runs of one Check that sat side by side: `check` rows of one `name` only. */
  | {
      kind: "checks";
      id: string;
      rows: SessionRow[];
    }
  /** A turn Fleet started: its notice, then the agent's tool rows and messages. Its last agent message is the next item. `handled` is the agent having done anything after it. */
  | {
      kind: "fleet_turn";
      id: string;
      notice: SessionRow;
      rows: SessionRow[];
      handled: boolean;
    }
  /** A `show_sketch` call whose sketch the ledger holds. */
  | ({
      kind: "sketch";
    } & ThreadSketch)
  /** A `show_file` call whose file the ledger holds, opened by the ledger's own path. */
  | {
      kind: "file";
      id: string;
      at: string;
      path: string;
      name: string;
    }
  /** A document revision the Job kept, placed at when it was kept. */
  | ({
      kind: "document";
    } & ThreadDocument)
  /** The person changed a value of the Job, read from the timeline. */
  | {
      kind: "change";
      id: string;
      at: string;
      what: ChangedValue;
      from: string;
      to: string;
    }
  /** The Job's agent never started. */
  | {
      kind: "not_started";
      id: string;
      at: string;
      cause: string;
      log?: string[];
    }
  /** The approved plan, after the item of the last row written before its `plan` row, only where that item is a `row`. */
  | ({
      kind: "plan";
    } & ThreadPlan)
  /** A Judge finished reading, after the last row written by then. `id` is `judged:<reading id>`; `clean` is a reading with no notes. */
  | {
      kind: "judged";
      id: string;
      at: string;
      artifact: JudgeArtifact;
      clean: boolean;
    };

/** A sketch card. `drawing` is the scene the call drew, with the `kept` hash of each mock node the ledger's kept scene draws alike. */
export type ThreadSketch = {
  id: string;
  at: string;
  /** The ledger's id, `sketch:<title>`. */
  sketch: string;
  title: string;
  drawing: unknown;
  /** The title of the sketch it is read as a change to. */
  against?: string;
  /** That sketch as it was drawn then: the latest earlier card of the title, else the scene the ledger kept at the call. */
  against_drawing?: unknown;
  /** A later card holds the same sketch. **Only the latest card carries how it was put and answered.** */
  replaced?: boolean;
  asked?: boolean;
  withdrawn?: boolean;
  settled?: CardSettled;
  /** Each drawing Fleet kept under the title, oldest first, on the latest card only. **Absent until every one reads.** */
  revisions?: unknown[];
};

/** A document card: one per page, its newest revision. */
export type ThreadDocument = {
  /** `document:<page>:r<revision>`. */
  id: string;
  at: string;
  page: string;
  title: string;
  revision: number;
  asked?: boolean;
  settled?: CardSettled;
};

/** Where a plan's card stands. `id` is `plan:v<version>`; `state` is `waiting`, `set_aside` or `approved`. */
export type ThreadPlan = {
  id: string;
  version: number;
  state: string;
};

/** How the person answered a sketch or a document put to him. `how` is `approved` or `sent_back`. */
export type CardSettled = {
  how: string;
  at: string;
  words?: string;
};

/** A value of a Job the person can change from its panel: `workflow`, `model` or `effort`. */
export type ChangedValue = string;
