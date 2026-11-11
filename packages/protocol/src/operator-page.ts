// The Operator's page — Bridge's mirror of `crates/ipc/src/operator_page.rs`.
// `crates/ipc/operations/operator_page.toml`, `operator_answer.toml`, `operator_later.toml`,
// `operator_turn_order.toml`, `operator.page_changed.toml` and `operator_in_front.toml`.

import type { OperatorRead, WaitingAct, WaitingItem } from "./sessions";

/** Whose act an item is: `decision` is his to decide, `go_ahead` is what the Operator would do itself, waiting on his word. */
export type OperatorTurnKind = "decision" | "go_ahead";

/** How soon an item wants him, decided by Fleet: `urgent` is a red main, `decision` his to decide, `go_ahead` what the Operator would do itself. */
export type OperatorUrgency = "urgent" | "decision" | "go_ahead";

/** His press on an item. Later is `operator_later`, and Ask is the chat opened with `talk_to_operator` carrying the item. */
export type OperatorAnswer = "yes" | "always_here" | "always" | "no";

/** What follows filing an issue. */
export type OperatorThen = "just_file" | "file_and_start" | "file_and_hand";

/** A live Job an issue would be handed to. */
export type OperatorHandTo = {
  session_id: string;
  job: string;
};

/** One Then offered on a go-ahead to file an issue. */
export type OperatorThenOffer = {
  then: OperatorThen;
  /** On `file_and_hand` only: the Job. That choice is offered only where one is live. */
  hand_to?: OperatorHandTo;
  /** The Operator's read marks this one as its pick. Absent otherwise. */
  pick?: boolean;
};

/** One item of Your turn. */
export type OperatorTurnItem = {
  /** `turn:<n>`, the id every act on the page names. */
  id: string;
  kind: OperatorTurnKind;
  /** How soon it wants him: `urgent` on a red main, else `decision` or `go_ahead` as its kind. */
  urgency: OperatorUrgency;
  /** The Job whose list holds it. Absent on a go-ahead no Job holds and on a red main. */
  session_id?: string;
  /** That Job's title. */
  job?: string;
  /** The repository's id, where it has one. */
  manifest_id?: string;
  /** The repository's name, where Fleet serves it. */
  repository?: string;
  /** The item as it was raised. Its `source` is its kind, `text` the thing, `act` and `options` what it asks. */
  item: WaitingItem;
  /** What the Operator made of it and what it would pick. Absent where no call read it. An empty `pick` is none. */
  read?: OperatorRead;
  /** RFC 3339. When it joined the queue. */
  raised_at: string;
  /** RFC 3339. Last in the queue until then. */
  later_until?: string;
  /** On a go-ahead to file an issue: the Thens offered, in the order to draw them. Absent otherwise. */
  then?: OperatorThenOffer[];
  /** Whether Always here and Always are offered. Absent where they are not. */
  keeps?: boolean;
};

/** What the Operator is reading now. */
export type OperatorOnIt = {
  /** What it is reading, in a few words. */
  reading: string;
  /** RFC 3339. */
  since: string;
};

/** A pull request that merged since he last looked. */
export type OperatorLanded = {
  session_id: string;
  /** The Job that held it. */
  job: string;
  number: string;
  title: string;
  /** RFC 3339. When it merged. */
  at: string;
};

/** One thing the Operator did on its own since he last looked. */
export type OperatorHandledAct = {
  /** RFC 3339. */
  at: string;
  /** What it was about: a Job's title, an item's first line. */
  title: string;
  /** Written `Kind: what`, as the Log's rows are. */
  outcome: string;
};

/** Where the week's Log is: the journal Fleet keeps for the Operator, a markdown file for each ISO week. */
export type OperatorLogFile = {
  /** `2026-W46`. */
  week: string;
  /** The file's absolute path on this machine. */
  path: string;
};

/** What the Operator handled itself since he last looked. */
export type OperatorHandled = {
  /** Newest first. Absent when none. */
  acts?: OperatorHandledAct[];
  /** The week's Log, where anything is written in it yet. */
  log?: OperatorLogFile;
};

/** A live Job. */
export type OperatorRunning = {
  session_id: string;
  title: string;
  repository?: string;
};

/** A Job waiting to start. */
export type OperatorComingUp = {
  session_id: string;
  title: string;
  /** What it waits on, in words. */
  needs: string;
};

/** One rule the Operator keeps from an Always or an Always here. */
export type OperatorLearned = {
  id: number;
  /** The repository it holds in. Absent on an Always, which holds everywhere. */
  manifest_id?: string;
  repository?: string;
  /** The rule in Fleet's words, such as `Start a fix when main is red`. */
  said: string;
  /** RFC 3339. */
  at: string;
};

/** One hold that stands over paths: only the Job it names writes under them, until it pushes or ends. */
export type OperatorHold = {
  /** The order's number, which `remove_operator_order` and the chat's `lift` name. */
  id: number;
  /** The repository it holds in. */
  manifest_id: string;
  repository?: string;
  /** The hold in Fleet's words, such as `Hold src/a until Fix the clock pushes`. */
  said: string;
  /** RFC 3339. */
  at: string;
};

/** Since he last looked, and what stands. A section with nothing in it is absent. */
export type OperatorDigest = {
  landed?: OperatorLanded[];
  handled: OperatorHandled;
  running?: OperatorRunning[];
  coming_up?: OperatorComingUp[];
  /** The rules, across projects. Rules only: a hold is not one. */
  learned?: OperatorLearned[];
  /** The holds that stand, beside Learned. */
  holds?: OperatorHold[];
};

/** How a repository's main stands on CI, as `main_ci_state` of the merge line has it. A red a newer commit's run is still on is `red`. */
export type OperatorMainState = "green" | "red" | "running" | "nothing_ran";

/** One repository's main, as the head draws it: a pill, red when main is red. */
export type OperatorMain = {
  manifest_id: string;
  /** The repository's name, as `OperatorTurnItem` gives it. */
  repository: string;
  /** Absent where Fleet has not read main yet. */
  state?: OperatorMainState;
  /** The commit the state was decided from, whole: the red's own on a red. Absent with `state`. */
  commit?: string;
  /** RFC 3339. When main first read red, kept while it stays red. `red` only. */
  since?: string;
};

/** The slots in use across every repository's pool. */
export type OperatorSlots = {
  /** Held by a Job, a process or work stranded in it, or under a take or a release. */
  used: number;
  /** Every slot of every pool, free, unmade or closed ones too. */
  total: number;
};

/** `operator_page` (`GET /operator/page`): the whole page. */
export type OperatorPage = {
  /** Whether the Operator is on. Off, `turn` is empty. */
  on: boolean;
  /** The page's one line, rewritten each time he comes back. Absent before the first. */
  top?: string;
  /** RFC 3339. When he last looked: what the digest counts from. */
  looked_at?: string;
  /** Present while the Operator is reading something to review. */
  on_it?: OperatorOnIt;
  /** Your turn, in the order he left it, with what he put Later last. */
  turn: OperatorTurnItem[];
  digest: OperatorDigest;
  /** Each repository Fleet serves, first first. Absent when none. */
  mains?: OperatorMain[];
  /** Absent where no pool is read. */
  slots?: OperatorSlots;
};

/** `operator_answer` (`POST /operator/page/answer`): one press on an item. */
export type OperatorAnswerPress = {
  /** The item's `id`. */
  id: string;
  answer: OperatorAnswer;
  /** On a go-ahead to file an issue: which Then. Absent takes the Operator's pick, else `just_file`. */
  then?: OperatorThen;
  /** The label of the option he chose, one the item lists in `options`. Absent takes the Operator's pick. */
  option?: string;
};

/** What `operator_answer` did. */
export type OperatorAnswered = {
  /** What Bridge opens: the act of an item Fleet cannot do itself. The item stays in Your turn until its Job's list drops it. */
  open?: WaitingAct;
  /** The Job a fix or a filed issue started. */
  started?: string;
  /** The issue filed, as its tracker names it. */
  filed?: string;
  /** The live Job the issue was handed to. */
  handed_to?: string;
  /** The page as `operator_page` serves it after the press. */
  page: OperatorPage;
};

/** `operator_turn_order` (`POST /operator/page/order`): the items as he dragged them. */
export type OperatorTurnOrder = {
  /** Item ids, first to last. Items it does not name follow, as they were. */
  ids: string[];
};

/** `operator_later` (`POST /operator/page/later`): one item goes last for the rest of the local day. */
export type OperatorLater = {
  id: string;
};

/** `operator.page_changed`: the page is not what it was, so read it again. Nothing else rides. */
export type OperatorPageChanged = Record<string, never>;

/** `operator_in_front` (`POST /operator/front`): Bridge's window came to the front. */
export type OperatorInFront = {
  /** True where he came back after time away and the page's top line is being rewritten. */
  rewriting: boolean;
};
